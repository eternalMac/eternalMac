#[derive(Debug, Clone)]
pub struct ServerInput {
    pub default_session: String,
    pub existing_sessions: Vec<String>,
    pub tailscale_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerState {
    pub actions: Vec<String>,
    pub healthy: bool,
}

pub fn reconcile_server(input: ServerInput) -> ServerState {
    if !input.tailscale_ready {
        return ServerState {
            actions: vec![],
            healthy: false,
        };
    }
    if input
        .existing_sessions
        .iter()
        .any(|name| name == &input.default_session)
    {
        return ServerState {
            actions: vec![],
            healthy: true,
        };
    }
    ServerState {
        actions: vec![format!("tmux:new-session:{}", input.default_session)],
        healthy: true,
    }
}
