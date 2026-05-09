use eternalmac::setup::client::build_client_plan;
use eternalmac::setup::server::build_server_plan;

#[test]
fn server_plan_has_default_session_and_brew_tools() {
    let plan = build_server_plan("mac-mini");
    assert_eq!(plan.default_session, "default");
    assert_eq!(
        plan.brew_packages,
        vec!["et", "tmux", "mutagen", "tailscale-app"]
    );
}

#[test]
fn client_plan_records_paired_server() {
    let plan = build_client_plan("mac-mini.tailnet");
    assert_eq!(plan.paired_server, "mac-mini.tailnet");
}
