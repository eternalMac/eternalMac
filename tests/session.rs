use std::cell::RefCell;

use eternalmac::app::paths::Paths;
use eternalmac::commands::session::{create_with, list_with, pin_session_with, unpin_session_with};
use eternalmac::config::store::Store;
use eternalmac::model::config::{ClientConfig, Config, Role, ServerConfig, SessionConfig};
use eternalmac::process::runner::{Output, Runner};
use eternalmac::session::service::{pin, unpin};
use eternalmac::tooling::et::{build_remote_command_args, build_remote_command_args_with_options};
use eternalmac::tooling::tmux::{list_sessions_args, new_session_args};

struct FakeRunner {
    calls: RefCell<Vec<(String, Vec<String>)>>,
    output: Output,
}

impl FakeRunner {
    fn success(stdout: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            output: Output {
                stdout: stdout.into(),
                stderr: String::new(),
                success: true,
            },
        }
    }

    fn failure(stderr: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            output: Output {
                stdout: String::new(),
                stderr: stderr.into(),
                success: false,
            },
        }
    }

    fn failure_with_streams(stdout: &str, stderr: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            output: Output {
                stdout: stdout.into(),
                stderr: stderr.into(),
                success: false,
            },
        }
    }
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<Output> {
        self.calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));
        Ok(self.output.clone())
    }
}

fn save_config(store: &Store, boot_sessions: Option<Vec<String>>, pinned: Option<Vec<String>>) {
    let role = if boot_sessions.is_some() && pinned.is_none() {
        Role::Server
    } else {
        Role::Client
    };
    store
        .save_config(&Config {
            role,
            server: boot_sessions.map(|boot_sessions| ServerConfig {
                host_label: "mac-mini".into(),
                default_session: "default".into(),
                boot_sessions,
                tailscale_dns: None,
            }),
            client: pinned.map(|pinned| ClientConfig {
                paired_server: "mac-mini".into(),
                server_ssh_user: None,
                server_etterminal_path: None,
                pinned,
                sync_pairs: vec![],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
}

fn save_client_config_with_connection(
    store: &Store,
    paired_server: &str,
    server_ssh_user: &str,
    server_etterminal_path: &str,
) {
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: paired_server.into(),
                server_ssh_user: Some(server_ssh_user.into()),
                server_etterminal_path: Some(server_etterminal_path.into()),
                pinned: vec![],
                sync_pairs: vec![],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
}

#[test]
fn pin_adds_missing_session() {
    let pinned = pin(vec!["default".into()], "pairing");
    assert_eq!(pinned, vec!["default", "pairing"]);
}

#[test]
fn pin_deduplicates_existing_session() {
    let pinned = pin(vec!["default".into()], "default");
    assert_eq!(pinned, vec!["default"]);
}

#[test]
fn unpin_removes_all_matching_sessions() {
    let pinned = unpin(
        vec!["default".into(), "pairing".into(), "default".into()],
        "default",
    );
    assert_eq!(pinned, vec!["pairing"]);
}

#[test]
fn unpin_is_noop_when_session_not_present() {
    let pinned = unpin(vec!["default".into()], "pairing");
    assert_eq!(pinned, vec!["default"]);
}

#[test]
fn session_list_runs_tmux_list_and_parses_output() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, Some(vec!["default".into()]), None);
    let runner = FakeRunner::success("default\npairing\n");

    let sessions = list_with(&store, &runner).unwrap();

    assert_eq!(sessions, vec!["default", "pairing"]);
    let calls = runner.calls.borrow();
    assert_eq!(calls.as_slice(), &[("tmux".into(), list_sessions_args())]);
}

#[test]
fn session_list_uses_et_against_paired_server_when_client_config_exists() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, None, Some(vec![]));
    let runner = FakeRunner::success(
        "prompt echo\n__ETERNALMAC_SESSIONS_BEGIN__\ndefault\npairing\n__ETERNALMAC_SESSIONS_END__\nSession terminated\n",
    );

    let sessions = list_with(&store, &runner).unwrap();

    assert_eq!(sessions, vec!["default", "pairing"]);
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "et".into(),
            build_remote_command_args(
                "mac-mini",
                "printf '__ETERNALMAC_SESSIONS_BEGIN__\\n'; tmux list-sessions -F '#S'; printf '__ETERNALMAC_SESSIONS_END__\\n'; exit"
            )
        )]
    );
}

#[test]
fn session_list_uses_persisted_terminal_path() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_connection(
        &store,
        "mac-mini",
        "devuser",
        "/opt/homebrew/bin/etterminal",
    );
    let runner = FakeRunner::success(
        "prompt echo\n__ETERNALMAC_SESSIONS_BEGIN__\ndefault\npairing\n__ETERNALMAC_SESSIONS_END__\nSession terminated\n",
    );

    let sessions = list_with(&store, &runner).unwrap();

    assert_eq!(sessions, vec!["default", "pairing"]);
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "et".into(),
            build_remote_command_args_with_options(
                "mac-mini",
                Some("devuser"),
                Some("/opt/homebrew/bin/etterminal"),
                "printf '__ETERNALMAC_SESSIONS_BEGIN__\\n'; tmux list-sessions -F '#S'; printf '__ETERNALMAC_SESSIONS_END__\\n'; exit"
            )
        )]
    );
}

#[test]
fn session_create_runs_tmux_new_session_locally_on_server() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, Some(vec!["default".into()]), None);
    let runner = FakeRunner::success("");

    create_with(&store, &runner, "demo").unwrap();

    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[("tmux".into(), new_session_args("demo"))]
    );
}

#[test]
fn session_create_uses_et_against_paired_server_when_client_config_exists() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, None, Some(vec![]));
    let runner = FakeRunner::success("");

    create_with(&store, &runner, "demo").unwrap();

    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "et".into(),
            build_remote_command_args("mac-mini", "tmux new-session -d -s 'demo'")
        )]
    );
}

#[test]
fn session_create_uses_persisted_terminal_path() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_connection(
        &store,
        "mac-mini",
        "devuser",
        "/opt/homebrew/bin/etterminal",
    );
    let runner = FakeRunner::success("");

    create_with(&store, &runner, "demo").unwrap();

    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "et".into(),
            build_remote_command_args_with_options(
                "mac-mini",
                Some("devuser"),
                Some("/opt/homebrew/bin/etterminal"),
                "tmux new-session -d -s 'demo'"
            )
        )]
    );
}

#[test]
fn session_create_returns_clear_error_on_non_zero_exit() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, Some(vec!["default".into()]), None);
    let runner = FakeRunner::failure("session exists");

    let error = create_with(&store, &runner, "demo").unwrap_err();
    assert!(error.to_string().contains("command failed: tmux"));
    assert!(error.to_string().contains("stderr: session exists"));
}

#[test]
fn remote_session_list_error_includes_stdout_when_stderr_is_empty() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, None, Some(vec![]));
    let runner = FakeRunner::failure_with_streams("unable to connect to paired server", "");

    let error = list_with(&store, &runner).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("command failed: et"));
    assert!(message.contains("stdout: unable to connect to paired server"));
}

#[test]
fn session_pin_persists_deduplicated_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(
        &store,
        Some(vec!["default".into()]),
        Some(vec!["default".into()]),
    );

    let pinned = pin_session_with(&store, "default").unwrap();
    assert_eq!(pinned, vec!["default"]);

    let pinned = pin_session_with(&store, "pairing").unwrap();
    assert_eq!(pinned, vec!["default", "pairing"]);

    let loaded = store.load_config().unwrap();
    assert_eq!(
        loaded.server.unwrap().boot_sessions,
        vec!["default", "pairing"]
    );
    assert_eq!(loaded.client.unwrap().pinned, vec!["default", "pairing"]);
}

#[test]
fn session_unpin_persists_updated_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(
        &store,
        Some(vec!["default".into(), "pairing".into()]),
        Some(vec!["default".into(), "pairing".into()]),
    );

    let pinned = unpin_session_with(&store, "default").unwrap();
    assert_eq!(pinned, vec!["pairing"]);

    let loaded = store.load_config().unwrap();
    assert_eq!(loaded.server.unwrap().boot_sessions, vec!["pairing"]);
    assert_eq!(loaded.client.unwrap().pinned, vec!["pairing"]);
}

#[test]
fn session_pin_updates_server_when_client_config_missing() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_config(&store, Some(vec!["default".into()]), None);

    pin_session_with(&store, "pairing").unwrap();

    let loaded = store.load_config().unwrap();
    assert_eq!(
        loaded.server.unwrap().boot_sessions,
        vec!["default", "pairing"]
    );
    assert!(loaded.client.is_none());
}
