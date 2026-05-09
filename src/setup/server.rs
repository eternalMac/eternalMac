#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPlan {
    pub role: &'static str,
    pub host_label: String,
    pub default_session: String,
    pub brew_packages: Vec<String>,
}

pub fn build_server_plan(host_label: &str) -> ServerPlan {
    ServerPlan {
        role: "server",
        host_label: host_label.to_string(),
        default_session: "default".into(),
        brew_packages: vec![
            "et".into(),
            "tmux".into(),
            "mutagen".into(),
            "tailscale-app".into(),
        ],
    }
}
