use eternalmac::tooling::brew::{install_cask_args, install_formula_args};
use eternalmac::tooling::tailscale::{detect_variant, parse_status_json, Variant};
use eternalmac::tooling::tmux::parse_sessions;

#[test]
fn brew_install_args_split_formulae_and_casks() {
    assert_eq!(
        install_formula_args(&["et".into(), "tmux".into()]),
        Some(vec!["install".into(), "et".into(), "tmux".into()])
    );
    assert_eq!(
        install_cask_args("tailscale-app"),
        vec!["install", "--cask", "tailscale-app"]
    );
}

#[test]
fn brew_install_args_reject_empty_formula_lists() {
    assert_eq!(install_formula_args(&[]), None);
}

#[test]
fn tailscale_status_parser_extracts_backend_state_and_dns() {
    let status = parse_status_json(
        r#"{
            "BackendState": "Running",
            "Self": { "DNSName": "mac-mini.example.ts.net" }
        }"#,
    )
    .unwrap();

    assert_eq!(status.backend_state, "Running");
    assert_eq!(status.dns_name.as_deref(), Some("mac-mini.example.ts.net"));
}

#[test]
fn tailscale_variant_detection_accepts_standalone_app() {
    let variant = detect_variant(&[
        "/Applications/Tailscale.app".into(),
        "/Applications/Something Else.app".into(),
    ]);

    assert_eq!(variant, Variant::Standalone);
}

#[test]
fn tmux_session_parser_ignores_blank_lines() {
    assert_eq!(
        parse_sessions("default\npairing\n\n"),
        vec!["default".to_string(), "pairing".to_string()]
    );
}
