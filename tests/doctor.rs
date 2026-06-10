use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::doctor::service::collect_issues;
use eternalmac::model::config::{ClientConfig, Config, Role, ServerConfig, SessionConfig};
use eternalmac::model::state::{State, SyncPairState};
use eternalmac::process::runner::{Output, Runner};
use eternalmac::tooling::ping::alive_check_args;
use predicates::str::contains;

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[derive(Debug, Default)]
struct FakeRunner {
    responses: BTreeMap<(String, Vec<String>), Output>,
    failing_programs: BTreeSet<String>,
    calls: RefCell<Vec<(String, Vec<String>)>>,
}

impl FakeRunner {
    fn with_response(mut self, program: &str, args: Vec<String>, output: Output) -> Self {
        self.responses.insert((program.to_string(), args), output);
        self
    }

    fn with_failing_program(mut self, program: &str) -> Self {
        self.failing_programs.insert(program.to_string());
        self
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.borrow().clone()
    }
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output> {
        self.calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));

        if self.failing_programs.contains(program) {
            return Err(anyhow!("command not found: {program}"));
        }

        if let Some(output) = self
            .responses
            .get(&(program.to_string(), args.to_vec()))
            .cloned()
        {
            return Ok(output);
        }

        Ok(Output {
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        })
    }
}

fn client_store_with_unreachable_server(tempdir: &tempfile::TempDir) -> Store {
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
            server_reachable: false,
            healthy: false,
            summary: "client daemon degraded: Eternal Terminal cannot reach paired server".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: current_unix_seconds(),
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![],
        })
        .unwrap();
    store
}

#[test]
fn doctor_reports_server_alive_when_host_pings_but_et_is_unreachable() {
    let tempdir = tempfile::tempdir().unwrap();
    let store = client_store_with_unreachable_server(&tempdir);
    let runner = FakeRunner::default().with_response(
        "ping",
        alive_check_args("mac-mini.example.ts.net"),
        Output {
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        },
    );

    let issues = collect_issues(&store, &runner).unwrap();

    assert!(issues.iter().any(|issue| issue
        .contains("paired server unreachable over Eternal Terminal: mac-mini.example.ts.net")));
    let alive_line = issues
        .iter()
        .find(|issue| issue.starts_with("server alive: yes"))
        .expect("expected a `server alive: yes` line");
    assert!(alive_line.contains("brew services restart et"));
}

#[test]
fn doctor_reports_server_not_alive_when_ping_gets_no_response() {
    let tempdir = tempfile::tempdir().unwrap();
    let store = client_store_with_unreachable_server(&tempdir);
    let runner = FakeRunner::default().with_response(
        "ping",
        alive_check_args("mac-mini.example.ts.net"),
        Output {
            stdout: String::new(),
            stderr: "Request timeout".into(),
            success: false,
            exit_code: Some(2),
        },
    );

    let issues = collect_issues(&store, &runner).unwrap();

    assert!(issues
        .iter()
        .any(|issue| issue.starts_with("server alive: no")));
}

#[test]
fn doctor_reports_server_alive_unknown_when_ping_fails_for_other_reasons() {
    let tempdir = tempfile::tempdir().unwrap();
    let store = client_store_with_unreachable_server(&tempdir);
    let runner = FakeRunner::default().with_response(
        "ping",
        alive_check_args("mac-mini.example.ts.net"),
        Output {
            stdout: String::new(),
            stderr: "ping: cannot resolve mac-mini.example.ts.net: Unknown host".into(),
            success: false,
            exit_code: Some(68),
        },
    );

    let issues = collect_issues(&store, &runner).unwrap();

    let unknown_line = issues
        .iter()
        .find(|issue| issue.starts_with("server alive: unknown"))
        .expect("expected a `server alive: unknown` line");
    assert!(unknown_line.contains("68"));
    assert!(unknown_line.contains("cannot resolve"));
}

#[test]
fn doctor_reports_server_alive_unknown_when_ping_cannot_run() {
    let tempdir = tempfile::tempdir().unwrap();
    let store = client_store_with_unreachable_server(&tempdir);
    let runner = FakeRunner::default().with_failing_program("ping");

    let issues = collect_issues(&store, &runner).unwrap();

    assert!(issues
        .iter()
        .any(|issue| issue.starts_with("server alive: unknown")));
}

#[test]
fn doctor_does_not_ping_when_server_is_reachable() {
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
            healthy: true,
            summary: "client daemon healthy".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: current_unix_seconds(),
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![],
        })
        .unwrap();
    let runner = FakeRunner::default();

    let issues = collect_issues(&store, &runner).unwrap();

    assert!(issues.is_empty());
    assert!(runner.calls().is_empty());
}

#[test]
fn doctor_reports_missing_config_on_unconfigured_machine() {
    let tempdir = tempfile::tempdir().unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .failure()
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
        .failure()
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
        .failure()
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
                ignore_paths: vec![],
                kind: None,
                label: None,
                status: "degraded".into(),
            }],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(contains("sync degraded: notes"));
}

#[test]
fn doctor_reports_client_server_unreachable() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: Some("devuser".into()),
                server_etterminal_path: Some("/opt/homebrew/bin/etterminal".into()),
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
            server_reachable: false,
            healthy: false,
            summary: "client daemon degraded: Eternal Terminal cannot reach paired server".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: current_unix_seconds(),
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(contains(
            "paired server unreachable over Eternal Terminal: mac-mini.example.ts.net",
        ))
        .stdout(contains("server alive:"));
}

#[test]
fn doctor_exits_success_when_no_issues_are_found() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: Some("devuser".into()),
                server_etterminal_path: Some("/opt/homebrew/bin/etterminal".into()),
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
            healthy: true,
            summary: "client daemon healthy".into(),
            tailscale_dns: Some("mac-mini.example.ts.net".into()),
            daemon_healthy: true,
            daemon_heartbeat_unix: current_unix_seconds(),
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![],
        })
        .unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains("no issues"));
}
