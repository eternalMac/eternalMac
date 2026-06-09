use std::cell::RefCell;
use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::model::config::{
    ClientConfig, Config, Role, ServerConfig, SessionConfig, SyncPairConfig,
};
use eternalmac::model::state::{State, SyncPairState};
use eternalmac::process::runner::{Output, Runner};
use eternalmac::tooling::et::build_remote_command_args_with_options;
use eternalmac::tooling::mutagen::list_args;
use eternalmac::tooling::tailscale::status_args;
use eternalmac::tooling::tmux::{list_sessions_args, new_session_args};

#[derive(Debug, Default)]
struct FakeRunner {
    responses: BTreeMap<(String, Vec<String>), Output>,
    calls: RefCell<Vec<(String, Vec<String>)>>,
}

impl FakeRunner {
    fn with_response(mut self, program: &str, args: Vec<String>, output: Output) -> Self {
        self.responses.insert((program.to_string(), args), output);
        self
    }
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output> {
        self.calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));

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
        })
    }
}

#[test]
fn server_run_once_refreshes_state_and_marks_daemon_healthy() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "tmux",
            list_sessions_args(),
            Output {
                stdout: "default\npairing\n".into(),
                stderr: String::new(),
                success: true,
            },
        );

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

    eternalmac::daemon::server::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert!(matches!(state.role, Role::Server));
    assert!(state.daemon_healthy);
    assert!(state.daemon_heartbeat_unix > 0);
    assert!(state.default_session_present);
    assert_eq!(state.known_sessions, vec!["default", "pairing"]);
    assert!(state.syncs.is_empty());

    let calls = runner.calls.borrow();
    assert!(calls
        .iter()
        .any(|(program, args)| program == "tailscale" && args == &status_args()));
    assert!(calls
        .iter()
        .any(|(program, args)| program == "tmux" && args == &list_sessions_args()));
}

#[test]
fn client_run_once_refreshes_sync_state_from_config_and_mutagen_listing() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "mutagen",
            list_args(),
            Output {
                stdout: "Name: project\nIdentifier: sync_project\nLabels: None\nAlpha:\n    URL: /Users/me/project\n    Connection state: Connected\nBeta:\n    URL: mac-mini.example.ts.net:~/project\n    Connection state: Connected\nStatus: Watching for changes\n".into(),
                stderr: String::new(),
                success: true,
            },
        );

    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![
                    SyncPairConfig {
                        name: "project".into(),
                        local: "/Users/me/project".into(),
                        remote: "mac-mini.example.ts.net:~/project".into(),
                        mode: "two-way-resolved".into(),
                        ignore_paths: vec![],
                        kind: None,
                        label: None,
                    },
                    SyncPairConfig {
                        name: "docs".into(),
                        local: "/Users/me/docs".into(),
                        remote: "mac-mini.example.ts.net:~/docs".into(),
                        mode: "two-way-resolved".into(),
                        ignore_paths: vec![],
                        kind: None,
                        label: None,
                    },
                ],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();

    eternalmac::daemon::client::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert!(matches!(state.role, Role::Client));
    assert!(state.daemon_healthy);
    assert!(state.daemon_heartbeat_unix > 0);
    assert_eq!(state.syncs.len(), 2);
    assert_eq!(state.syncs[0].name, "project");
    assert_eq!(state.syncs[0].local, "/Users/me/project");
    assert_eq!(state.syncs[0].remote, "mac-mini.example.ts.net:~/project");
    assert_eq!(state.syncs[0].mode, "two-way-resolved");
    assert_eq!(state.syncs[0].status, "active");
    assert_eq!(state.syncs[1].name, "docs");
    assert_eq!(state.syncs[1].status, "missing");

    let calls = runner.calls.borrow();
    assert!(calls
        .iter()
        .any(|(program, args)| program == "tailscale" && args == &status_args()));
    assert!(calls
        .iter()
        .any(|(program, args)| program == "mutagen" && args == &list_args()));
}

#[test]
fn client_run_once_marks_server_unreachable_when_et_probe_fails() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let et_probe_args = build_remote_command_args_with_options(
        "mac-mini.example.ts.net",
        Some("devuser"),
        Some("/opt/homebrew/bin/etterminal"),
        "true",
    );
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "mutagen",
            list_args(),
            Output {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "et",
            et_probe_args.clone(),
            Output {
                stdout: "unable to connect".into(),
                stderr: String::new(),
                success: false,
            },
        );

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

    eternalmac::daemon::client::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert!(state.tailscale_ok);
    assert!(!state.server_reachable);
    assert!(!state.healthy);
    assert!(state.summary.contains("Eternal Terminal"));

    let calls = runner.calls.borrow();
    assert!(calls
        .iter()
        .any(|(program, args)| program == "et" && args == &et_probe_args));
}

#[test]
fn client_run_once_matches_sync_names_exactly() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "mutagen",
            list_args(),
            Output {
                stdout: "Name: project\nIdentifier: sync_project\nLabels: None\nAlpha:\n    URL: /Users/me/project\n    Connection state: Connected\nBeta:\n    URL: mac-mini.example.ts.net:~/project\n    Connection state: Connected\nStatus: Watching for changes\n".into(),
                stderr: String::new(),
                success: true,
            },
        );

    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![
                    SyncPairConfig {
                        name: "proj".into(),
                        local: "/Users/me/proj".into(),
                        remote: "mac-mini.example.ts.net:~/proj".into(),
                        mode: "two-way-resolved".into(),
                        ignore_paths: vec![],
                        kind: None,
                        label: None,
                    },
                    SyncPairConfig {
                        name: "project".into(),
                        local: "/Users/me/project".into(),
                        remote: "mac-mini.example.ts.net:~/project".into(),
                        mode: "two-way-resolved".into(),
                        ignore_paths: vec![],
                        kind: None,
                        label: None,
                    },
                ],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();

    eternalmac::daemon::client::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert_eq!(state.syncs[0].name, "proj");
    assert_eq!(state.syncs[0].status, "missing");
    assert_eq!(state.syncs[1].name, "project");
    assert_eq!(state.syncs[1].status, "active");
}

#[test]
fn client_run_once_matches_syncs_by_name_and_endpoints_not_first_name_match() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "mutagen",
            list_args(),
            Output {
                stdout: "Name: project\nIdentifier: sync_wrong\nLabels: None\nAlpha:\n    URL: /Users/me/other-project\n    Connection state: Connected\nBeta:\n    URL: mac-mini.example.ts.net:~/other-project\n    Connection state: Connected\nStatus: Watching for changes\n\nName: project\nIdentifier: sync_right\nLabels: None\nAlpha:\n    URL: /Users/me/project\n    Connection state: Connected\nBeta:\n    URL: mac-mini.example.ts.net:~/project\n    Connection state: Connected\nStatus: reconnecting\n".into(),
                stderr: String::new(),
                success: true,
            },
        );

    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![SyncPairConfig {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "mac-mini.example.ts.net:~/project".into(),
                    mode: "two-way-resolved".into(),
                    ignore_paths: vec![],
                    kind: None,
                    label: None,
                }],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();

    eternalmac::daemon::client::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert_eq!(state.syncs.len(), 1);
    assert_eq!(state.syncs[0].name, "project");
    assert_eq!(state.syncs[0].status, "degraded");
    assert!(!state.healthy);
}

#[test]
fn client_failure_state_snapshot_marks_daemon_unhealthy() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![SyncPairConfig {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "mac-mini.example.ts.net:~/project".into(),
                    mode: "two-way-resolved".into(),
                    ignore_paths: vec![],
                    kind: None,
                    label: None,
                }],
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
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![SyncPairState {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
                mode: "two-way-resolved".into(),
                ignore_paths: vec![],
                kind: None,
                label: None,
                status: "active".into(),
            }],
        })
        .unwrap();

    eternalmac::daemon::client::save_failure_state(&store, &anyhow!("tailscale timed out"))
        .unwrap();

    let state = store.load_state().unwrap();
    assert!(!state.daemon_healthy);
    assert!(!state.healthy);
    assert!(state.daemon_heartbeat_unix > 0);
    assert!(state.summary.contains("tailscale timed out"));
    assert_eq!(state.syncs.len(), 1);
}

#[test]
fn server_failure_state_does_not_overwrite_client_role_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![SyncPairConfig {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "mac-mini.example.ts.net:~/project".into(),
                    mode: "two-way-resolved".into(),
                    ignore_paths: vec![],
                    kind: None,
                    label: None,
                }],
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
            daemon_heartbeat_unix: 42,
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![SyncPairState {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
                mode: "two-way-resolved".into(),
                ignore_paths: vec![],
                kind: None,
                label: None,
                status: "active".into(),
            }],
        })
        .unwrap();

    let error = eternalmac::daemon::server::save_failure_state(&store, &anyhow!("wrong daemon"))
        .unwrap_err();
    assert!(error.to_string().contains("config role"));

    let state = store.load_state().unwrap();
    assert!(matches!(state.role, Role::Client));
    assert_eq!(state.summary, "client daemon healthy");
    assert!(state.daemon_healthy);
}

#[test]
fn client_failure_state_does_not_overwrite_server_role_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
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
            daemon_heartbeat_unix: 42,
            default_session_present: true,
            known_sessions: vec!["default".into()],
            syncs: vec![],
        })
        .unwrap();

    let error = eternalmac::daemon::client::save_failure_state(&store, &anyhow!("wrong daemon"))
        .unwrap_err();
    assert!(error.to_string().contains("config role"));

    let state = store.load_state().unwrap();
    assert!(matches!(state.role, Role::Server));
    assert_eq!(state.summary, "server daemon healthy");
    assert!(state.daemon_healthy);
}

#[test]
fn server_run_once_reconciles_missing_default_session() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "tmux",
            list_sessions_args(),
            Output {
                stdout: "pairing\n".into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "tmux",
            new_session_args("default"),
            Output {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
            },
        );

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

    eternalmac::daemon::server::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert!(state.default_session_present);
    assert!(state.known_sessions.iter().any(|name| name == "default"));

    let calls = runner.calls.borrow();
    assert!(calls
        .iter()
        .any(|(program, args)| { program == "tmux" && args == &new_session_args("default") }));
}

#[test]
fn server_run_once_treats_tmux_no_server_as_empty_and_reconciles() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "tmux",
            list_sessions_args(),
            Output {
                stdout: String::new(),
                stderr: "no server running on /tmp/tmux-501/default".into(),
                success: false,
            },
        )
        .with_response(
            "tmux",
            new_session_args("default"),
            Output {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
            },
        );

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

    eternalmac::daemon::server::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert!(state.healthy);
    assert!(state.default_session_present);
    assert_eq!(state.known_sessions, vec!["default"]);
    assert!(state
        .summary
        .contains("reconciled missing default session 'default'"));

    let calls = runner.calls.borrow();
    assert!(calls
        .iter()
        .any(|(program, args)| { program == "tmux" && args == &new_session_args("default") }));
}

#[test]
fn client_run_once_marks_transient_sync_status_as_degraded() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "mutagen",
            list_args(),
            Output {
                stdout: "Name: project\nIdentifier: sync_project\nLabels: None\nAlpha:\n    URL: /Users/me/project\n    Connection state: Connected\nBeta:\n    URL: mac-mini.example.ts.net:~/project\n    Connection state: Connected\nStatus: reconnecting\n".into(),
                stderr: String::new(),
                success: true,
            },
        );

    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini.example.ts.net".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs: vec![SyncPairConfig {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "mac-mini.example.ts.net:~/project".into(),
                    mode: "two-way-resolved".into(),
                    ignore_paths: vec![],
                    kind: None,
                    label: None,
                }],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();

    eternalmac::daemon::client::run_once(&paths, &store, &runner).unwrap();

    let state = store.load_state().unwrap();
    assert!(!state.healthy);
    assert_eq!(state.syncs.len(), 1);
    assert_eq!(state.syncs[0].name, "project");
    assert_eq!(state.syncs[0].status, "degraded");
    assert_eq!(state.summary, "client daemon degraded");
}

// Real mutagen statuses observed during a normal transfer (mutagen 0.18.1):
// "Scanning files" and "Staging files on beta". These are in-progress phases of
// a healthy sync and must NOT be reported as degraded, or health flaps on every
// transfer. See https://github.com/eternalMac/eternalMac issue for details.
fn client_config_with_single_project_pair() -> Config {
    Config {
        role: Role::Client,
        server: None,
        client: Some(ClientConfig {
            paired_server: "mac-mini.example.ts.net".into(),
            server_ssh_user: None,
            server_etterminal_path: None,
            pinned: vec![],
            sync_pairs: vec![SyncPairConfig {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
                mode: "two-way-resolved".into(),
                ignore_paths: vec![],
                kind: None,
                label: None,
            }],
        }),
        session: SessionConfig { auto_attach: true },
    }
}

fn run_client_once_with_sync_status(status_line: &str) -> State {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default()
        .with_response(
            "tailscale",
            status_args(),
            Output {
                stdout:
                    r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                        .into(),
                stderr: String::new(),
                success: true,
            },
        )
        .with_response(
            "mutagen",
            list_args(),
            Output {
                stdout: format!(
                    "Name: project\nIdentifier: sync_project\nLabels: None\nAlpha:\n    URL: /Users/me/project\n    Connection state: Connected\nBeta:\n    URL: mac-mini.example.ts.net:~/project\n    Connection state: Connected\nStatus: {status_line}\n"
                ),
                stderr: String::new(),
                success: true,
            },
        );

    store
        .save_config(&client_config_with_single_project_pair())
        .unwrap();

    eternalmac::daemon::client::run_once(&paths, &store, &runner).unwrap();
    store.load_state().unwrap()
}

#[test]
fn client_run_once_treats_scanning_transfer_as_active() {
    let state = run_client_once_with_sync_status("Scanning files");

    assert_eq!(state.syncs[0].status, "active");
    assert!(state.healthy);
    assert_eq!(state.summary, "client daemon healthy");
}

#[test]
fn client_run_once_treats_staging_transfer_as_active() {
    let state = run_client_once_with_sync_status("Staging files on beta");

    assert_eq!(state.syncs[0].status, "active");
    assert!(state.healthy);
}

#[test]
fn client_run_once_still_marks_reconnecting_as_degraded() {
    let state = run_client_once_with_sync_status("reconnecting");

    assert_eq!(state.syncs[0].status, "degraded");
    assert!(!state.healthy);
}
