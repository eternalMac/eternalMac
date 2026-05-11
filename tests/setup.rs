use std::cell::RefCell;

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
    fn with_failure(program: &str, args: Vec<String>, stderr: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            stubs: vec![Stub {
                program: program.to_string(),
                args,
                output: Output {
                    stdout: String::new(),
                    stderr: stderr.to_string(),
                    success: false,
                },
            }],
        }
    }
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
    assert!(!state.server_reachable);
    assert!(!state.healthy);
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
                    "load".to_string(),
                    "-w".to_string(),
                    paths.server_plist.display().to_string(),
                ]
    }));
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
    assert!(!state.tailscale_ok);
    assert!(!state.server_reachable);
    assert!(!state.healthy);
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
                    "load".to_string(),
                    "-w".to_string(),
                    paths.client_plist.display().to_string(),
                ]
    }));
}

#[test]
fn server_setup_returns_error_and_skips_persistence_when_tmux_reports_failure() {
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

    assert!(!paths.config_file.exists());
    assert!(!paths.state_file.exists());
}

#[test]
fn client_setup_returns_error_and_skips_persistence_when_launchctl_reports_failure() {
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

    assert!(!paths.config_file.exists());
    assert!(!paths.state_file.exists());
}
