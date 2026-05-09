use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Inspector {
    installed: BTreeMap<String, bool>,
}

impl Inspector {
    pub fn from_installed<const N: usize>(items: [(&str, bool); N]) -> Self {
        let mut installed = BTreeMap::new();
        for (name, value) in items {
            installed.insert(name.to_string(), value);
        }
        Self { installed }
    }

    pub fn has(&self, name: &str) -> bool {
        self.installed.get(name).copied().unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyPlan {
    pub packages: Vec<String>,
}

pub fn build_dependency_plan(inspector: &Inspector) -> DependencyPlan {
    let required = ["et", "tmux", "mutagen", "tailscale"];
    let packages = required
        .into_iter()
        .filter(|name| !inspector.has(name))
        .map(String::from)
        .collect();
    DependencyPlan { packages }
}
