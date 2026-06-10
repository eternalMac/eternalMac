use eternalmac::tooling::brew::{install_cask_args, install_formula_args};
use eternalmac::tooling::ssh::{
    batch_login_check_args, build_sync_destination, et_server_probe_args,
    interactive_authorize_key_args, managed_identity_paths, port_probe_args,
    render_managed_host_block, upsert_managed_host_block, validate_ssh_host, validate_ssh_user,
};
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
        build_sync_destination("devuser", "mac-mini.example.ts.net", "~/project"),
        "devuser@mac-mini.example.ts.net:~/project"
    );
}

#[test]
fn ssh_validation_accepts_safe_tailnet_host_and_user() {
    assert!(validate_ssh_host("mac-mini.example.ts.net").is_ok());
    assert!(validate_ssh_user("devuser").is_ok());
    assert!(validate_ssh_user("dev.user-1").is_ok());
}

#[test]
fn ssh_validation_rejects_values_that_can_corrupt_ssh_config() {
    assert!(validate_ssh_host("mac-mini\nHost evil").is_err());
    assert!(validate_ssh_host("devuser@mac-mini").is_err());
    assert!(validate_ssh_host("mac-mini:22").is_err());
    assert!(validate_ssh_user("devuser\nIdentityFile /tmp/key").is_err());
    assert!(validate_ssh_user("dev:user").is_err());
    assert!(validate_ssh_user("dev@user").is_err());
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

#[test]
fn et_server_probe_retries_local_port_2022() {
    let args = et_server_probe_args();

    assert_eq!(args[0], "-c");
    assert!(args[1].contains("nc -G 2 -z localhost 2022"));
    assert!(args[1].contains("sleep 1"));
}

#[test]
fn managed_identity_paths_are_scoped_by_user_and_host() {
    let ssh_dir = std::path::Path::new("/Users/me/.ssh");
    let paths = managed_identity_paths(ssh_dir, "mac-mini.example.ts.net", "devuser");

    assert_eq!(
        paths.private_key_path,
        ssh_dir.join("eternalmac_devuser_mac_mini_example_ts_net_ed25519")
    );
    assert_eq!(
        paths.public_key_path,
        ssh_dir.join("eternalmac_devuser_mac_mini_example_ts_net_ed25519.pub")
    );
}

#[test]
fn managed_host_block_renders_expected_ssh_config() {
    let block = render_managed_host_block(
        "mac-mini.example.ts.net",
        "devuser",
        "/Users/me/.ssh/eternalmac_devuser_mac_mini_example_ts_net_ed25519",
    );

    assert!(block.contains("# >>> eternalmac mac-mini.example.ts.net >>>"));
    assert!(block.contains("Host mac-mini.example.ts.net"));
    assert!(block.contains("User devuser"));
    assert!(block.contains(
        "IdentityFile /Users/me/.ssh/eternalmac_devuser_mac_mini_example_ts_net_ed25519"
    ));
    assert!(block.contains("IdentitiesOnly yes"));
}

#[test]
fn upsert_managed_host_block_prepends_new_block() {
    let existing = "Host github.com\n  User git\n";
    let block = render_managed_host_block("mac-mini.example.ts.net", "devuser", "/tmp/key");

    let updated = upsert_managed_host_block(existing, "mac-mini.example.ts.net", &block);

    assert!(updated.starts_with("# >>> eternalmac mac-mini.example.ts.net >>>"));
    assert!(updated.contains("Host github.com"));
}

#[test]
fn upsert_managed_host_block_replaces_existing_block_in_place() {
    let original = "\
# >>> eternalmac mac-mini.example.ts.net >>>\n\
Host mac-mini.example.ts.net\n\
  User olduser\n\
# <<< eternalmac mac-mini.example.ts.net <<<\n\
\n\
Host github.com\n\
  User git\n";
    let replacement = render_managed_host_block("mac-mini.example.ts.net", "devuser", "/tmp/key");

    let updated = upsert_managed_host_block(original, "mac-mini.example.ts.net", &replacement);

    assert!(updated.contains("User devuser"));
    assert!(!updated.contains("User olduser"));
    assert!(updated.contains("Host github.com"));
}

#[test]
fn ssh_batch_login_check_uses_batch_mode_and_true_probe() {
    assert_eq!(
        batch_login_check_args("mac-mini.example.ts.net"),
        vec![
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
            "-o".to_string(),
            "ConnectTimeout=5".to_string(),
            "mac-mini.example.ts.net".to_string(),
            "true".to_string(),
        ]
    );
}

#[test]
fn interactive_authorize_key_args_disable_pubkey_and_send_remote_command() {
    let args = interactive_authorize_key_args(
        "devuser",
        "mac-mini.example.ts.net",
        "ssh-ed25519 AAAAB3Nza key-comment",
    );

    assert_eq!(args[0], "-o");
    assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
    assert!(args.contains(&"PreferredAuthentications=password,keyboard-interactive".to_string()));
    assert!(args.contains(&"PubkeyAuthentication=no".to_string()));
    assert!(args.contains(&"devuser@mac-mini.example.ts.net".to_string()));
    assert!(args.last().unwrap().contains("authorized_keys"));
    assert!(args
        .last()
        .unwrap()
        .contains("ssh-ed25519 AAAAB3Nza key-comment"));
}

#[test]
fn interactive_authorize_key_args_neutralize_injection_in_public_key() {
    // The public key is interpolated into a remote shell command that appends
    // to authorized_keys. A key carrying shell metacharacters must stay inside
    // a single-quoted literal so it cannot break out and execute.
    let args = interactive_authorize_key_args(
        "devuser",
        "mac-mini.example.ts.net",
        "ssh-ed25519 AAAA'; rm -rf ~ #",
    );
    let remote_command = args.last().unwrap();

    assert!(remote_command.contains("'ssh-ed25519 AAAA'\\''; rm -rf ~ #'"));
    // The unescaped break-out fragment must never appear.
    assert!(!remote_command.contains("AAAA'; rm"));
}
