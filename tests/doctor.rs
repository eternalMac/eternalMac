use assert_cmd::Command;
use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::model::config::{ClientConfig, Config, Role, ServerConfig, SessionConfig};
use eternalmac::model::state::{State, SyncPairState};
use predicates::str::contains;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn doctor_reports_missing_config_on_unconfigured_machine() {
    let tempdir = tempfile::tempdir().unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains(
            "config missing: run `eternalMac setup server` or `eternalMac setup client`",
        ));
}

#[test]
fn doctor_reports_missing_state_when_config_exists() {
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

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains(
            "state missing: re-run `eternalMac setup server` to restore local state",
        ));
}

#[test]
fn doctor_reports_stale_daemon_heartbeat() {
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
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains("daemon heartbeat stale"));
}

#[test]
fn doctor_reports_degraded_sync_state_for_client() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
    store
        .save_state(&State {
            role: Role::Client,
            tailscale_ok: true,
            server_reachable: true,
            healthy: false,
            summary: "client daemon degraded".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: current_unix_seconds(),
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![SyncPairState {
                name: "notes".into(),
                local: "/Users/me/notes".into(),
                remote: "mac-mini.example.ts.net:~/notes".into(),
                mode: "two-way-resolved".into(),
                status: "degraded".into(),
            }],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains("sync degraded: notes"));
}
