#[derive(Debug, Clone)]
pub struct ClientInput {
    pub pinned_sessions: Vec<String>,
    pub tailscale_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientState {
    pub auto_attach: Vec<String>,
    pub healthy: bool,
}

pub fn reconcile_client(input: ClientInput) -> ClientState {
    if !input.tailscale_ready {
        return ClientState {
            auto_attach: vec![],
            healthy: false,
        };
    }
    ClientState {
        auto_attach: input.pinned_sessions,
        healthy: true,
    }
}
