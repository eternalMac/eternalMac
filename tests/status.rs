use assert_cmd::Command;
use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::model::config::{ClientConfig, Config, Role, ServerConfig, SessionConfig};
use eternalmac::model::state::{State, SyncPairState};
use eternalmac::status::service::{render_summary, StatusSnapshot};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn status_fails_with_setup_guidance_when_unconfigured() {
    let tempdir = tempfile::tempdir().unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["status"])
        .assert()
        .failure()
        .stderr(
            contains("status requires setup")
                .and(contains("eternalMac setup server"))
                .and(contains("eternalMac setup client")),
        );
}

#[test]
fn status_renders_persisted_summary_when_configured() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    store
        .save_config(&Config {
            role: Role::Server,
            server: Some(ServerConfig {
                host_label: "mac-mini".into(),
                default_session: "default".into(),
                boot_sessions: vec!["default".into()],
                tailscale_dns: Some("mac-mini.example.ts.net".into()),
            }),
            client: None,
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
    store
        .save_state(&State {
            role: Role::Server,
            tailscale_ok: true,
            server_reachable: true,
            healthy: true,
            summary: "server daemon healthy".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: current_unix_seconds(),
            default_session_present: true,
            known_sessions: vec!["default".into(), "pair".into()],
            syncs: vec![],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(
            contains("role: server")
                .and(contains("summary: server daemon healthy"))
                .and(contains("known sessions: default, pair")),
        );
}

#[test]
fn status_fails_when_config_and_state_roles_do_not_match() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    store
        .save_config(&Config {
            role: Role::Server,
            server: Some(ServerConfig {
                host_label: "mac-mini".into(),
                default_session: "default".into(),
                boot_sessions: vec!["default".into()],
                tailscale_dns: Some("mac-mini.example.ts.net".into()),
            }),
            client: None,
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
    store
        .save_state(&State {
            role: Role::Client,
            tailscale_ok: true,
            server_reachable: true,
            healthy: true,
            summary: "stale client state".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: 1_700_000_000,
            default_session_present: true,
            known_sessions: vec!["default".into()],
            syncs: vec![],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["status"])
        .assert()
        .failure()
        .stderr(
            contains("status config/state role mismatch")
                .and(contains("config=server"))
                .and(contains("state=client"))
                .and(contains("eternalMac setup server")),
        );
}

#[test]
fn status_marks_stale_daemon_heartbeat_as_unhealthy() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    store
        .save_config(&Config {
            role: Role::Server,
            server: Some(ServerConfig {
                host_label: "mac-mini".into(),
                default_session: "default".into(),
                boot_sessions: vec!["default".into()],
                tailscale_dns: Some("mac-mini.example.ts.net".into()),
            }),
            client: None,
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
    store
        .save_state(&State {
            role: Role::Server,
            tailscale_ok: true,
            server_reachable: true,
            healthy: true,
            summary: "server daemon healthy".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: 1,
            default_session_present: true,
            known_sessions: vec!["default".into()],
            syncs: vec![],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(
            contains("healthy: no")
                .and(contains("daemon healthy: no"))
                .and(contains("daemon heartbeat stale: yes")),
        );
}

#[test]
fn client_status_renders_dotsync_targets_separately() {
    let snapshot = StatusSnapshot {
        config: Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini".into(),
                server_ssh_user: Some("devuser".into()),
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![],
            }),
            session: SessionConfig { auto_attach: true },
        },
        state: State {
            role: Role::Client,
            tailscale_ok: true,
            server_reachable: true,
            healthy: true,
            summary: "client setup complete; runtime health pending".into(),
            tailscale_dns: None,
            daemon_healthy: false,
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![
                SyncPairState {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "devuser@mac-mini:~/project".into(),
                    mode: "two-way-resolved".into(),
                    status: "created".into(),
                    ignore_paths: vec![],
                    kind: None,
                    label: None,
                },
                SyncPairState {
                    name: "dotsync-claude".into(),
                    local: "/Users/me/.claude".into(),
                    remote: "devuser@mac-mini:~/.claude".into(),
                    mode: "two-way-resolved".into(),
                    status: "created".into(),
                    ignore_paths: vec!["auth.json".into()],
                    kind: Some("dotsync".into()),
                    label: Some("Claude Code".into()),
                },
            ],
        },
    };

    let rendered = render_summary(&snapshot);

    assert!(rendered.contains("syncs: project:created"));
    assert!(rendered.contains("dotsync: Claude Code:created"));
}
