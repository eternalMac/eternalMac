use eternalmac::tooling::brew::{install_cask_args, install_formula_args};
use eternalmac::tooling::ssh::{build_sync_destination, port_probe_args};
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
fn tailscale_variant_detection_detects_app_store_variant() {
    let variant = detect_variant(&[
        "/Applications/Tailscale.app".into(),
        "/Applications/Tailscale (App Store).app".into(),
    ]);

    assert_eq!(variant, Variant::AppStore);
}

#[test]
fn tailscale_variant_detection_detects_open_source_variant() {
    let variant = detect_variant(&[
        "/opt/homebrew/bin/tailscaled".into(),
        "/usr/local/bin/tailscale".into(),
    ]);

    assert_eq!(variant, Variant::OpenSource);
}

#[test]
fn tailscale_variant_detection_returns_unknown_without_markers() {
    let variant = detect_variant(&["/Applications/Other.app".into()]);

    assert_eq!(variant, Variant::Unknown);
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

#[test]
fn ssh_sync_destination_uses_user_host_and_path() {
    assert_eq!(
        build_sync_destination("kindshadow", "mac-mini.example.ts.net", "~/project"),
        "kindshadow@mac-mini.example.ts.net:~/project"
    );
}

#[test]
fn ssh_port_probe_uses_nc_with_host_and_port_22() {
    assert_eq!(
        port_probe_args("mac-mini.example.ts.net"),
        vec![
            "-G".to_string(),
            "5".to_string(),
            "-z".to_string(),
            "mac-mini.example.ts.net".to_string(),
            "22".to_string(),
        ]
    );
}
