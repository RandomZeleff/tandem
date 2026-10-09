//! Mojang library/argument rules (`allow`/`disallow` by OS, arch and launcher features).

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    // `version` (a regex on the OS version) is only used for old macOS quirks; ignored.
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: HashMap<String, bool>,
}

/// The machine and launcher capabilities rules are evaluated against.
#[derive(Debug, Clone)]
pub struct Environment {
    /// Mojang OS name: `windows`, `osx` or `linux`.
    pub os_name: &'static str,
    /// Mojang arch name: `x86` (32-bit), `x86_64` or `arm64`.
    pub arch: &'static str,
    /// Enabled launcher features (`has_custom_resolution`, `is_demo_user`, …).
    pub features: HashMap<String, bool>,
}

impl Environment {
    pub fn current() -> Self {
        let os_name = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };
        let arch = if cfg!(target_arch = "x86") {
            "x86"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x86_64"
        };
        Self {
            os_name,
            arch,
            features: HashMap::new(),
        }
    }

    /// An Intel Mac: what Apple Silicon runs through Rosetta 2 for versions with no
    /// arm64 Java or LWJGL natives.
    pub fn rosetta() -> Self {
        Self {
            os_name: "osx",
            arch: "x86_64",
            features: HashMap::new(),
        }
    }

    pub fn is_rosetta(&self) -> bool {
        cfg!(all(target_os = "macos", target_arch = "aarch64")) && self.arch == "x86_64"
    }

    pub fn is_64bit(&self) -> bool {
        self.arch != "x86"
    }
}

impl Rule {
    fn matches(&self, env: &Environment) -> bool {
        if let Some(os) = &self.os {
            if os.name.as_deref().is_some_and(|n| n != env.os_name) {
                return false;
            }
            if os.arch.as_deref().is_some_and(|a| a != env.arch) {
                return false;
            }
        }
        self.features
            .iter()
            .all(|(name, wanted)| env.features.get(name).copied().unwrap_or(false) == *wanted)
    }
}

/// No rules means allowed; otherwise the last matching rule decides (default: disallowed).
pub fn is_allowed(rules: &[Rule], env: &Environment) -> bool {
    if rules.is_empty() {
        return true;
    }
    rules
        .iter()
        .rfind(|rule| rule.matches(env))
        .is_some_and(|rule| rule.action == RuleAction::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(os_name: &'static str, arch: &'static str) -> Environment {
        Environment {
            os_name,
            arch,
            features: HashMap::new(),
        }
    }

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn allow_all_except_osx() {
        let r = rules(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#);
        assert!(is_allowed(&r, &env("windows", "x86_64")));
        assert!(!is_allowed(&r, &env("osx", "arm64")));
    }

    #[test]
    fn only_for_one_os() {
        let r = rules(r#"[{"action":"allow","os":{"name":"linux"}}]"#);
        assert!(is_allowed(&r, &env("linux", "x86_64")));
        assert!(!is_allowed(&r, &env("windows", "x86_64")));
    }

    #[test]
    fn arch_and_features() {
        let x86 = rules(r#"[{"action":"allow","os":{"arch":"x86"}}]"#);
        assert!(is_allowed(&x86, &env("windows", "x86")));
        assert!(!is_allowed(&x86, &env("windows", "x86_64")));

        let demo = rules(r#"[{"action":"allow","features":{"is_demo_user":true}}]"#);
        let mut e = env("windows", "x86_64");
        assert!(!is_allowed(&demo, &e));
        e.features.insert("is_demo_user".into(), true);
        assert!(is_allowed(&demo, &e));
    }
}
