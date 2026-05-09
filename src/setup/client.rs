#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientPlan {
    pub role: &'static str,
    pub paired_server: String,
    pub brew_packages: Vec<String>,
}

pub fn build_client_plan(server: &str) -> ClientPlan {
    ClientPlan {
        role: "client",
        paired_server: server.to_string(),
        brew_packages: vec![
            "et".into(),
            "tmux".into(),
            "mutagen".into(),
            "tailscale-app".into(),
        ],
    }
}
