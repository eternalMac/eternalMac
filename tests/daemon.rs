use eternalmac::daemon::server::{reconcile_server, ServerInput};

#[test]
fn server_reconcile_creates_default_session_when_missing() {
    let state = reconcile_server(ServerInput {
        default_session: "default".into(),
        existing_sessions: vec![],
        tailscale_ready: true,
    });

    assert_eq!(state.actions, vec!["tmux:new-session:default"]);
}
