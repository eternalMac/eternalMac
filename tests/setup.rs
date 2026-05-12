use std::cell::RefCell;
use std::fs;

use anyhow::Result;
use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::process::runner::{Output, Runner};
use eternalmac::setup::client::{apply_client_setup, ClientSetupInput, SyncRootInput};
use eternalmac::setup::server::apply_server_setup;

#[derive(Debug, Clone)]
struct Stub {
    program: String,
    args: Vec<String>,
    output: Output,
}

#[derive(Default)]
struct FakeRunner {
    calls: RefCell<Vec<(String, Vec<String>)>>,
    stubs: Vec<Stub>,
}

impl FakeRunner {
    fn with_stubs(stubs: Vec<Stub>) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            stubs,
        }
    }

    fn with_failure(program: &str, args: Vec<String>, stderr: &str) -> Self {
        Self::with_stubs(vec![Stub {
            program: program.to_string(),
            args,
            output: Output {
                stdout: String::new(),
                stderr: stderr.to_string(),
                success: false,
            },
        }])
    }
}

fn call_index(calls: &[(String, Vec<String>)], program: &str, args: &[String]) -> Option<usize> {
    calls.iter().position(|(called_program, called_args)| {
        called_program == program && called_args.as_slice() == args
    })
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output> {
        self.calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));

        if let Some(stub) = self
            .stubs
            .iter()
            .find(|stub| stub.program == program && stub.args == args)
        {
            return Ok(stub.output.clone());
        }

        let stdout = match (program, args.first().map(String::as_str)) {
            ("tailscale", Some("status")) => {
                r#"{"BackendState":"Running","Self":{"DNSName":"mac-mini.example.ts.net"}}"#
                    .to_string()
            }
            _ => String::new(),
        };

        Ok(Output {
            stdout,
            stderr: String::new(),
            success: true,
        })
    }
}

#[test]
fn server_setup_writes_config_state_launch_agent_and_bootstrap_session() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default();

    let summary = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap();

    let config = store.load_config().unwrap();
    assert!(matches!(
        config.role,
        eternalmac::model::config::Role::Server
    ));
    assert_eq!(config.server.as_ref().unwrap().host_label, "mac-mini");
    assert_eq!(
        config.server.as_ref().unwrap().tailscale_dns.as_deref(),
        Some("mac-mini.example.ts.net")
    );

    let state = store.load_state().unwrap();
    assert_eq!(
        state.tailscale_dns.as_deref(),
        Some("mac-mini.example.ts.net")
    );
    assert!(state.tailscale_ok);
    assert!(state.server_reachable);
    assert!(state.healthy);
    assert_eq!(
        state.summary,
        "server setup complete; runtime health pending"
    );
    assert!(!state.daemon_healthy);
    assert_eq!(state.known_sessions, vec!["default"]);
    assert!(state.default_session_present);

    assert!(paths.server_plist.exists());
    assert_eq!(summary.dns_name, "mac-mini.example.ts.net");
    assert_eq!(summary.default_session, "default");

    let calls = runner.calls.borrow();
    assert!(calls.iter().any(|(program, args)| {
        program == "brew"
            && args
                == &vec![
                    "install".to_string(),
                    "et".to_string(),
                    "tmux".to_string(),
                    "mutagen".to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "brew"
            && args
                == &vec![
                    "install".to_string(),
                    "--cask".to_string(),
                    "tailscale-app".to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "tailscale" && args == &vec!["status".to_string(), "--json".to_string()]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "tmux"
            && args
                == &vec![
                    "new-session".to_string(),
                    "-d".to_string(),
                    "-s".to_string(),
                    "default".to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "unload".to_string(),
                    "-w".to_string(),
                    paths.client_plist.display().to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "load".to_string(),
                    "-w".to_string(),
                    paths.server_plist.display().to_string(),
                ]
    }));

    let tmux_args = vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        "default".to_string(),
    ];
    let unload_client_args = vec![
        "unload".to_string(),
        "-w".to_string(),
        paths.client_plist.display().to_string(),
    ];
    let launchctl_args = vec![
        "load".to_string(),
        "-w".to_string(),
        paths.server_plist.display().to_string(),
    ];
    let tmux_index = call_index(&calls, "tmux", &tmux_args).unwrap();
    let unload_index = call_index(&calls, "launchctl", &unload_client_args).unwrap();
    let launchctl_index = call_index(&calls, "launchctl", &launchctl_args).unwrap();
    assert!(unload_index < launchctl_index);
    assert!(tmux_index < launchctl_index);
}

#[test]
fn server_setup_errors_when_tailscale_dns_is_unavailable() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_stubs(vec![Stub {
        program: "tailscale".to_string(),
        args: vec!["status".to_string(), "--json".to_string()],
        output: Output {
            stdout: r#"{"BackendState":"Running","Self":{}}"#.to_string(),
            stderr: String::new(),
            success: true,
        },
    }]);

    let err = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("tailscale"));
    assert!(err_text.contains("dns"));

    assert!(!paths.config_file.exists());
    assert!(!paths.state_file.exists());

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "tmux"));
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}

#[test]
fn client_setup_persists_sync_pairs_and_creates_mutagen_sessions() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default();

    let summary = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![SyncRootInput {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
            }],
        },
    )
    .unwrap();

    let config = store.load_config().unwrap();
    assert!(matches!(
        config.role,
        eternalmac::model::config::Role::Client
    ));
    assert_eq!(
        config.client.as_ref().unwrap().paired_server,
        "mac-mini.example.ts.net"
    );
    assert_eq!(config.client.as_ref().unwrap().sync_pairs.len(), 1);
    assert_eq!(
        config.client.as_ref().unwrap().sync_pairs[0].mode,
        "two-way-resolved"
    );

    let state = store.load_state().unwrap();
    assert!(state.tailscale_ok);
    assert!(state.server_reachable);
    assert!(state.healthy);
    assert_eq!(
        state.summary,
        "client setup complete; runtime health pending"
    );
    assert!(!state.daemon_healthy);
    assert_eq!(state.syncs.len(), 1);
    assert_eq!(state.syncs[0].name, "project");
    assert_eq!(state.syncs[0].local, "/Users/me/project");
    assert_eq!(state.syncs[0].remote, "mac-mini.example.ts.net:~/project");
    assert_eq!(state.syncs[0].mode, "two-way-resolved");
    assert_eq!(state.syncs[0].status, "created");

    assert!(paths.client_plist.exists());
    assert_eq!(summary.paired_server, "mac-mini.example.ts.net");
    assert_eq!(summary.sync_names, vec!["project"]);

    let calls = runner.calls.borrow();
    assert!(calls.iter().any(|(program, args)| {
        program == "brew"
            && args
                == &vec![
                    "install".to_string(),
                    "et".to_string(),
                    "tmux".to_string(),
                    "mutagen".to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "brew"
            && args
                == &vec![
                    "install".to_string(),
                    "--cask".to_string(),
                    "tailscale-app".to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "tailscale" && args == &vec!["status".to_string(), "--json".to_string()]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "mutagen"
            && args
                == &vec![
                    "sync".to_string(),
                    "create".to_string(),
                    "--name".to_string(),
                    "project".to_string(),
                    "--sync-mode".to_string(),
                    "two-way-resolved".to_string(),
                    "/Users/me/project".to_string(),
                    "mac-mini.example.ts.net:~/project".to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "unload".to_string(),
                    "-w".to_string(),
                    paths.server_plist.display().to_string(),
                ]
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "load".to_string(),
                    "-w".to_string(),
                    paths.client_plist.display().to_string(),
                ]
    }));

    let mutagen_args = vec![
        "sync".to_string(),
        "create".to_string(),
        "--name".to_string(),
        "project".to_string(),
        "--sync-mode".to_string(),
        "two-way-resolved".to_string(),
        "/Users/me/project".to_string(),
        "mac-mini.example.ts.net:~/project".to_string(),
    ];
    let unload_server_args = vec![
        "unload".to_string(),
        "-w".to_string(),
        paths.server_plist.display().to_string(),
    ];
    let launchctl_args = vec![
        "load".to_string(),
        "-w".to_string(),
        paths.client_plist.display().to_string(),
    ];
    let mutagen_index = call_index(&calls, "mutagen", &mutagen_args).unwrap();
    let unload_index = call_index(&calls, "launchctl", &unload_server_args).unwrap();
    let launchctl_index = call_index(&calls, "launchctl", &launchctl_args).unwrap();
    assert!(unload_index < launchctl_index);
    assert!(mutagen_index < launchctl_index);
}

#[test]
fn server_setup_fails_when_client_unload_returns_unexpected_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_failure(
        "launchctl",
        vec![
            "unload".to_string(),
            "-w".to_string(),
            paths.client_plist.display().to_string(),
        ],
        "permission denied",
    );

    let err = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("launchctl"));
    assert!(err_text.contains("unload"));
    assert!(err_text.contains("permission denied"));

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "load".to_string(),
                    "-w".to_string(),
                    paths.server_plist.display().to_string(),
                ]
    }));
}

#[test]
fn server_setup_ignores_client_unload_not_loaded_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_stubs(vec![Stub {
        program: "launchctl".to_string(),
        args: vec![
            "unload".to_string(),
            "-w".to_string(),
            paths.client_plist.display().to_string(),
        ],
        output: Output {
            stdout: String::new(),
            stderr: "Could not find specified service".to_string(),
            success: false,
        },
    }]);

    let summary = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap();
    assert_eq!(summary.dns_name, "mac-mini.example.ts.net");

    let calls = runner.calls.borrow();
    assert!(calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "load".to_string(),
                    "-w".to_string(),
                    paths.server_plist.display().to_string(),
                ]
    }));
}

#[test]
fn client_setup_fails_when_server_unload_returns_unexpected_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_failure(
        "launchctl",
        vec![
            "unload".to_string(),
            "-w".to_string(),
            paths.server_plist.display().to_string(),
        ],
        "permission denied",
    );

    let err = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![SyncRootInput {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
            }],
        },
    )
    .unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("launchctl"));
    assert!(err_text.contains("unload"));
    assert!(err_text.contains("permission denied"));

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "load".to_string(),
                    "-w".to_string(),
                    paths.client_plist.display().to_string(),
                ]
    }));
}

#[test]
fn client_setup_ignores_server_unload_not_loaded_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_stubs(vec![Stub {
        program: "launchctl".to_string(),
        args: vec![
            "unload".to_string(),
            "-w".to_string(),
            paths.server_plist.display().to_string(),
        ],
        output: Output {
            stdout: String::new(),
            stderr: "Could not find specified service".to_string(),
            success: false,
        },
    }]);

    let summary = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![SyncRootInput {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
            }],
        },
    )
    .unwrap();
    assert_eq!(summary.sync_names, vec!["project"]);

    let calls = runner.calls.borrow();
    assert!(calls.iter().any(|(program, args)| {
        program == "launchctl"
            && args
                == &vec![
                    "load".to_string(),
                    "-w".to_string(),
                    paths.client_plist.display().to_string(),
                ]
    }));
}

#[test]
fn server_setup_returns_error_before_launchctl_when_tmux_reports_failure() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_failure(
        "tmux",
        vec![
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            "default".to_string(),
        ],
        "tmux failed",
    );

    let err = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("tmux"));
    assert!(err_text.contains("new-session"));
    assert!(err_text.contains("tmux failed"));

    let config = store.load_config().unwrap();
    assert!(matches!(
        config.role,
        eternalmac::model::config::Role::Server
    ));
    let state = store.load_state().unwrap();
    assert!(matches!(
        state.role,
        eternalmac::model::config::Role::Server
    ));

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}

#[test]
fn server_setup_launchctl_failure_does_not_persist_completed_healthy_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_failure(
        "launchctl",
        vec![
            "load".to_string(),
            "-w".to_string(),
            paths.server_plist.display().to_string(),
        ],
        "launch failed",
    );

    let err = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("launchctl"));
    assert!(err_text.contains("load"));
    assert!(err_text.contains("launch failed"));

    let state = store.load_state().unwrap();
    assert!(!state.healthy);
    assert_ne!(
        state.summary,
        "server setup complete; runtime health pending"
    );
}

#[test]
fn server_setup_errors_when_config_snapshot_read_fails_with_non_not_found() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    fs::create_dir_all(paths.config_file.parent().unwrap()).unwrap();
    fs::create_dir_all(&paths.config_file).unwrap();
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default();

    let err = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("snapshot"));
    assert!(err_text.contains("config"));

    assert!(paths.config_file.is_dir());
    assert!(!paths.state_file.exists());

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "tmux"));
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}

#[test]
fn server_setup_rolls_back_config_when_state_write_fails() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    fs::create_dir_all(paths.state_dir.parent().unwrap()).unwrap();
    fs::write(&paths.state_dir, "not-a-directory").unwrap();
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default();

    let err = apply_server_setup(&paths, &store, &runner, "mac-mini".into()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains("state"));

    assert!(!paths.config_file.exists());
    assert!(!paths.state_file.exists());

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "tmux"));
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}

#[test]
fn client_setup_returns_error_and_keeps_persisted_state_when_launchctl_reports_failure() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_failure(
        "launchctl",
        vec![
            "load".to_string(),
            "-w".to_string(),
            paths.client_plist.display().to_string(),
        ],
        "launch failed",
    );

    let err = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![SyncRootInput {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
            }],
        },
    )
    .unwrap_err();

    let err_text = err.to_string();
    assert!(err_text.contains("launchctl"));
    assert!(err_text.contains("load"));
    assert!(err_text.contains("launch failed"));

    let config = store.load_config().unwrap();
    assert!(matches!(
        config.role,
        eternalmac::model::config::Role::Client
    ));
    let state = store.load_state().unwrap();
    assert!(state.tailscale_ok);
    assert!(!state.healthy);
    assert_ne!(
        state.summary,
        "client setup complete; runtime health pending"
    );
}

#[test]
fn client_setup_mutagen_failure_after_partial_creation_persists_progress() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());
    let runner = FakeRunner::with_stubs(vec![Stub {
        program: "mutagen".into(),
        args: vec![
            "sync".to_string(),
            "create".to_string(),
            "--name".to_string(),
            "documents".to_string(),
            "--sync-mode".to_string(),
            "two-way-resolved".to_string(),
            "/Users/me/documents".to_string(),
            "mac-mini.example.ts.net:~/documents".to_string(),
        ],
        output: Output {
            stdout: String::new(),
            stderr: "mutagen failed".to_string(),
            success: false,
        },
    }]);

    let err = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![
                SyncRootInput {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "mac-mini.example.ts.net:~/project".into(),
                },
                SyncRootInput {
                    name: "documents".into(),
                    local: "/Users/me/documents".into(),
                    remote: "mac-mini.example.ts.net:~/documents".into(),
                },
            ],
        },
    )
    .unwrap_err();

    let err_text = err.to_string();
    assert!(err_text.contains("mutagen"));
    assert!(err_text.contains("sync"));
    assert!(err_text.contains("mutagen failed"));

    let state = store.load_state().unwrap();
    assert_eq!(state.syncs.len(), 2);
    assert_eq!(state.syncs[0].name, "project");
    assert_eq!(state.syncs[0].status, "created");
    assert_eq!(state.syncs[1].name, "documents");
    assert_eq!(state.syncs[1].status, "pending");
    assert!(!state.healthy);

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}

#[test]
fn client_setup_errors_when_config_snapshot_read_fails_with_non_not_found() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    fs::create_dir_all(paths.config_file.parent().unwrap()).unwrap();
    fs::create_dir_all(&paths.config_file).unwrap();
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default();

    let err = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![SyncRootInput {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
            }],
        },
    )
    .unwrap_err();

    let err_text = err.to_string();
    assert!(err_text.contains("snapshot"));
    assert!(err_text.contains("config"));

    assert!(paths.config_file.is_dir());
    assert!(!paths.state_file.exists());

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "mutagen"));
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}

#[test]
fn client_setup_rolls_back_config_when_state_write_fails() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    fs::create_dir_all(paths.state_dir.parent().unwrap()).unwrap();
    fs::write(&paths.state_dir, "not-a-directory").unwrap();
    let store = Store::new(paths.clone());
    let runner = FakeRunner::default();

    let err = apply_client_setup(
        &paths,
        &store,
        &runner,
        ClientSetupInput {
            paired_server: "mac-mini.example.ts.net".into(),
            sync_roots: vec![SyncRootInput {
                name: "project".into(),
                local: "/Users/me/project".into(),
                remote: "mac-mini.example.ts.net:~/project".into(),
            }],
        },
    )
    .unwrap_err();

    let err_text = err.to_string();
    assert!(err_text.contains("state"));

    assert!(!paths.config_file.exists());
    assert!(!paths.state_file.exists());

    let calls = runner.calls.borrow();
    assert!(!calls.iter().any(|(program, _)| program == "mutagen"));
    assert!(!calls.iter().any(|(program, _)| program == "launchctl"));
}
