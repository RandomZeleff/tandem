//! Dev tool: two launchers in one process become friends through the real public relays,
//! share their presence, invite each other, then part.
//!
//! cargo run -p tandem-core --example friends -- selftest

use std::time::Duration;

use iroh::SecretKey;
use tandem_core::db::Database;
use tandem_core::friends::service::{Event, Friends, Me};
use tandem_core::friends::{FriendState, Presence};

fn me(name: &str) -> Me {
    Me {
        name: name.into(),
        mc_uuid: format!("{name}-uuid"),
        avatar: None,
    }
}

async fn wait_for<F: Fn(&Event) -> bool>(
    events: &mut tokio::sync::broadcast::Receiver<Event>,
    what: &str,
    matches: F,
) -> Result<Event, Box<dyn std::error::Error>> {
    let found = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            match events.recv().await {
                Ok(event) if matches(&event) => return Some(event),
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
    })
    .await
    .ok()
    .flatten()
    .ok_or(format!("timed out waiting for {what}"))?;
    println!("  ✓ {what}");
    Ok(found)
}

async fn until<F, Fut>(what: &str, check: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..120 {
        if check().await {
            println!("  ✓ {what}");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(format!("timed out waiting for {what}").into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("selftest") {
        return Err("usage: friends selftest".into());
    }
    let http = tandem_core::context::http_client()?;
    let tmp = tempfile::tempdir()?;
    let db_a = Database::open(&tmp.path().join("a.db")).await?;
    let db_b = Database::open(&tmp.path().join("b.db")).await?;

    let a = Friends::start(
        db_a.clone(),
        http.clone(),
        SecretKey::generate(),
        me("Alice"),
        Some(Presence::Online),
    )
    .await?;
    let b = Friends::start(
        db_b.clone(),
        http.clone(),
        SecretKey::generate(),
        me("Bob"),
        Some(Presence::Online),
    )
    .await?;
    println!("Alice {} · Bob {}", a.code(), b.code());
    let mut events_a = a.subscribe();
    let mut events_b = b.subscribe();

    // The code is published in the background: give it a moment.
    let mut added = None;
    for _ in 0..20 {
        match a.add(&http, &b.code()).await {
            Ok(friend) => {
                added = Some(friend);
                break;
            }
            Err(err) => {
                println!("  … {err}");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    let added = added.ok_or("Bob's code never resolved")?;
    println!("  ✓ Alice found Bob by his code ({:?})", added.state);

    let request = wait_for(&mut events_b, "Bob receives Alice's request", |e| {
        matches!(e, Event::Request { .. })
    })
    .await?;
    let Event::Request { id, name } = request else {
        unreachable!()
    };
    assert_eq!(name, "Alice");
    b.accept(&id).await?;

    until("both are friends", || async {
        let a_ok = a
            .list()
            .await
            .map(|l| {
                l.iter()
                    .any(|f| f.friend.state == FriendState::Friend && f.friend.name == "Bob")
            })
            .unwrap_or(false);
        let b_ok = b
            .list()
            .await
            .map(|l| {
                l.iter()
                    .any(|f| f.friend.state == FriendState::Friend && f.friend.name == "Alice")
            })
            .unwrap_or(false);
        a_ok && b_ok
    })
    .await?;

    b.set_presence(Some(Presence::Playing {
        instance: "Prominence II".into(),
    }))
    .await;
    until("Alice sees Bob playing", || async {
        a.list()
            .await
            .map(|l| {
                l.iter().any(|f| {
                    f.presence
                        == Some(Presence::Playing {
                            instance: "Prominence II".into(),
                        })
                })
            })
            .unwrap_or(false)
    })
    .await?;

    let alice_id = a.id().to_string();
    let bob_id = b.id().to_string();
    a.invite(
        &bob_id,
        "7K2P-QX9M",
        Some("Cobbleverse".into()),
        Some("Monde test".into()),
    )
    .await?;
    let invite = wait_for(&mut events_b, "Bob receives the invite", |e| {
        matches!(e, Event::Invite { .. })
    })
    .await?;
    println!("    {invite:?}");

    b.set_presence(None).await;
    until("Bob appears offline", || async {
        a.list()
            .await
            .map(|l| l.iter().all(|f| f.presence.is_none()))
            .unwrap_or(false)
    })
    .await?;

    b.remove(&alice_id).await?;
    until("Alice no longer has Bob", || async {
        a.list().await.map(|l| l.is_empty()).unwrap_or(false)
    })
    .await?;
    let _ = wait_for(&mut events_a, "Alice was told", |e| {
        matches!(e, Event::Changed)
    })
    .await;

    a.shutdown().await;
    b.shutdown().await;
    println!("selftest ok");
    Ok(())
}
