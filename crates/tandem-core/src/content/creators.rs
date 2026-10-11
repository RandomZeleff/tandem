//! The people behind an instance's content, with their support links (Phase 4.8).
//!
//! Modrinth gives support links per project, not per author: projects are grouped by
//! organization when they have one, otherwise by the owner of their team. Three batched
//! requests whatever the number of mods (projects, teams, organizations).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::modrinth::{self, DonationUrl};
use crate::error::Result;
use crate::Context;

/// Ids per request: keeps URLs short for packs with hundreds of mods.
const BATCH: usize = 100;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupportLink {
    /// `Ko-fi`, `Patreon`… as the author named it; may be empty.
    pub platform: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub name: String,
    pub avatar_url: Option<String>,
    /// Their Modrinth page.
    pub url: String,
    pub organization: bool,
    /// Titles of their projects in this instance.
    pub projects: Vec<String>,
    pub support: Vec<SupportLink>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Creators {
    /// Modrinth projects in the instance.
    pub projects: usize,
    /// Creators who accept support first, then by number of projects.
    pub creators: Vec<Creator>,
}

#[derive(Debug, Deserialize)]
struct ProjectTeam {
    id: String,
    title: String,
    team: String,
    #[serde(default)]
    organization: Option<String>,
    #[serde(default)]
    donation_urls: Vec<DonationUrl>,
}

#[derive(Debug, Deserialize)]
struct Member {
    team_id: String,
    user: MemberUser,
    #[serde(default)]
    is_owner: bool,
    #[serde(default)]
    ordering: i64,
    #[serde(default = "yes")]
    accepted: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct MemberUser {
    id: String,
    username: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Org {
    id: String,
    slug: String,
    name: String,
    #[serde(default)]
    icon_url: Option<String>,
}

/// Creators of the Modrinth content installed in an instance.
pub async fn of_instance(ctx: &Context, instance_id: &str) -> Result<Creators> {
    let mut ids: Vec<String> = ctx
        .db
        .list_content(instance_id)
        .await?
        .into_iter()
        .map(|c| c.project_id)
        .collect();
    ids.sort();
    ids.dedup();
    of_projects(ctx, &ids).await
}

/// Creators of these Modrinth projects (unknown ids are skipped).
pub async fn of_projects(ctx: &Context, ids: &[String]) -> Result<Creators> {
    let mut projects: Vec<ProjectTeam> = Vec::new();
    for chunk in ids.chunks(BATCH) {
        projects
            .extend(modrinth::bulk::<ProjectTeam>(ctx, modrinth::API, "projects", chunk).await?);
    }
    let teams: Vec<String> = unique(
        projects
            .iter()
            .filter(|p| p.organization.is_none())
            .map(|p| &p.team),
    );
    let mut members: Vec<Member> = Vec::new();
    for chunk in teams.chunks(BATCH) {
        let lists: Vec<Vec<Member>> = modrinth::bulk(ctx, modrinth::API_V3, "teams", chunk).await?;
        members.extend(lists.into_iter().flatten());
    }
    let orgs: Vec<String> = unique(projects.iter().filter_map(|p| p.organization.as_ref()));
    let mut organizations: Vec<Org> = Vec::new();
    for chunk in orgs.chunks(BATCH) {
        organizations
            .extend(modrinth::bulk::<Org>(ctx, modrinth::API_V3, "organizations", chunk).await?);
    }
    Ok(group(projects, members, organizations))
}

fn unique<'a>(values: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values.filter(|v| seen.insert(*v)).cloned().collect()
}

/// The team's owner: the member Modrinth marks as owner, else the first accepted one.
fn owners(members: Vec<Member>) -> HashMap<String, MemberUser> {
    let mut by_team: HashMap<String, Vec<Member>> = HashMap::new();
    for m in members.into_iter().filter(|m| m.accepted) {
        by_team.entry(m.team_id.clone()).or_default().push(m);
    }
    by_team
        .into_iter()
        .filter_map(|(team, mut list)| {
            list.sort_by_key(|m| (!m.is_owner, m.ordering));
            list.into_iter().next().map(|m| (team, m.user))
        })
        .collect()
}

fn group(projects: Vec<ProjectTeam>, members: Vec<Member>, organizations: Vec<Org>) -> Creators {
    let owners = owners(members);
    let orgs: HashMap<&str, &Org> = organizations.iter().map(|o| (o.id.as_str(), o)).collect();
    let total = projects.len();
    let mut order: Vec<String> = Vec::new();
    let mut creators: HashMap<String, Creator> = HashMap::new();
    for p in projects {
        let (key, creator) = match p.organization.as_deref().and_then(|id| orgs.get(id)) {
            Some(org) => (
                format!("org:{}", org.id),
                Creator {
                    name: org.name.clone(),
                    avatar_url: org.icon_url.clone(),
                    url: format!("https://modrinth.com/organization/{}", org.slug),
                    organization: true,
                    projects: Vec::new(),
                    support: Vec::new(),
                },
            ),
            None => match owners.get(&p.team) {
                Some(user) => (
                    format!("user:{}", user.id),
                    Creator {
                        name: user.username.clone(),
                        avatar_url: user.avatar_url.clone(),
                        url: format!("https://modrinth.com/user/{}", user.username),
                        organization: false,
                        projects: Vec::new(),
                        support: Vec::new(),
                    },
                ),
                // A team Modrinth did not return: the project stands for its creator.
                None => (
                    format!("project:{}", p.id),
                    Creator {
                        name: p.title.clone(),
                        avatar_url: None,
                        url: format!("https://modrinth.com/project/{}", p.id),
                        organization: false,
                        projects: Vec::new(),
                        support: Vec::new(),
                    },
                ),
            },
        };
        let entry = creators.entry(key.clone()).or_insert_with(|| {
            order.push(key);
            creator
        });
        entry.projects.push(p.title);
        for d in p.donation_urls {
            if d.url.starts_with("http") && !entry.support.iter().any(|s| s.url == d.url) {
                entry.support.push(SupportLink {
                    platform: d.platform,
                    url: d.url,
                });
            }
        }
    }
    let mut creators: Vec<Creator> = order
        .into_iter()
        .filter_map(|k| creators.remove(&k))
        .collect();
    // Stable: equal creators keep Modrinth's order.
    creators.sort_by_key(|c| (c.support.is_empty(), std::cmp::Reverse(c.projects.len())));
    for c in &mut creators {
        c.projects.sort_by_key(|t| t.to_lowercase());
    }
    Creators {
        projects: total,
        creators,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: &str, team: &str, org: Option<&str>, donations: &[&str]) -> ProjectTeam {
        ProjectTeam {
            id: id.into(),
            title: format!("Mod {id}"),
            team: team.into(),
            organization: org.map(Into::into),
            donation_urls: donations
                .iter()
                .map(|u| DonationUrl {
                    platform: "Ko-fi".into(),
                    url: (*u).into(),
                })
                .collect(),
        }
    }

    fn member(team: &str, user: &str, owner: bool, ordering: i64) -> Member {
        Member {
            team_id: team.into(),
            user: MemberUser {
                id: user.into(),
                username: user.into(),
                avatar_url: None,
            },
            is_owner: owner,
            ordering,
            accepted: true,
        }
    }

    #[test]
    fn groups_by_organization_then_team_owner() {
        let projects = vec![
            project("a", "t1", None, &["https://ko-fi.com/alice"]),
            project("b", "t2", None, &["https://ko-fi.com/alice"]),
            project("c", "t3", Some("o1"), &[]),
            project("d", "t4", Some("o1"), &["https://example.org/donate"]),
            project("e", "t5", None, &[]),
            project("f", "t6", None, &[]),
        ];
        let members = vec![
            member("t1", "helper", false, 0),
            member("t1", "alice", true, 1),
            member("t2", "alice", true, 0),
            member("t5", "bob", false, 2),
            member("t5", "carol", false, 1),
        ];
        let orgs = vec![Org {
            id: "o1".into(),
            slug: "caffeine".into(),
            name: "Caffeine".into(),
            icon_url: None,
        }];
        let c = group(projects, members, orgs);
        assert_eq!(c.projects, 6);
        let names: Vec<_> = c.creators.iter().map(|c| c.name.as_str()).collect();
        // Support first (alice: 2 projects, Caffeine: 2), then carol, then the project
        // whose team is unknown.
        assert_eq!(names, ["alice", "Caffeine", "carol", "Mod f"]);
        assert_eq!(c.creators[0].support.len(), 1, "same link once");
        assert_eq!(c.creators[0].projects, ["Mod a", "Mod b"]);
        assert!(c.creators[1].organization);
        assert_eq!(
            c.creators[1].url,
            "https://modrinth.com/organization/caffeine"
        );
    }
}
