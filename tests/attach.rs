use std::cell::RefCell;

use eternalmac::app::paths::Paths;
use eternalmac::commands::attach::{resolve_session_name, run_with};
use eternalmac::config::store::Store;
use eternalmac::model::config::{ClientConfig, Config, Role, SessionConfig};
use eternalmac::process::runner::{Output, Runner};
use eternalmac::tooling::et::{
    build_attach_args, build_attach_args_with_options, build_new_session_args,
    build_new_session_args_with_options,
};

struct FakeRunner {
    calls: RefCell<Vec<(String, Vec<String>)>>,
    interactive_calls: RefCell<Vec<(String, Vec<String>)>>,
    output: Output,
}

impl FakeRunner {
    fn success() -> Self {
        Self {
            calls: RefCell::new(vec![]),
            interactive_calls: RefCell::new(vec![]),
            output: Output {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
            },
        }
    }

    fn failure(stderr: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            interactive_calls: RefCell::new(vec![]),
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
            interactive_calls: RefCell::new(vec![]),
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

    fn run_interactive(&self, program: &str, args: &[String]) -> anyhow::Result<Output> {
        self.interactive_calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));
        Ok(self.output.clone())
    }
}

fn save_client_config(store: &Store, paired_server: &str) {
    save_client_config_with_connection(store, paired_server, None, None);
}

fn save_client_config_with_connection(
    store: &Store,
    paired_server: &str,
    server_ssh_user: Option<&str>,
    server_etterminal_path: Option<&str>,
) {
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: paired_server.into(),
                server_ssh_user: server_ssh_user.map(str::to_string),
                server_etterminal_path: server_etterminal_path.map(str::to_string),
                pinned: vec![],
                sync_pairs: vec![],
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
}

#[test]
fn attach_args_target_named_tmux_session() {
    assert_eq!(
        build_attach_args("mac-mini", "default"),
        vec!["mac-mini", "-c", "tmux attach -t 'default'"]
    );
}

#[test]
fn attach_args_quote_session_names_with_spaces() {
    assert_eq!(
        build_attach_args("mac-mini", "pair programming"),
        vec!["mac-mini", "-c", "tmux attach -t 'pair programming'"]
    );
}

#[test]
fn attach_args_escape_single_quotes_in_session_names() {
    assert_eq!(
        build_attach_args("mac-mini", "dev's session"),
        vec!["mac-mini", "-c", "tmux attach -t 'dev'\\''s session'"]
    );
}

#[test]
fn attach_args_neutralize_command_injection_in_session_names() {
    // A session name carrying shell metacharacters must stay confined to a
    // single-quoted literal so the remote shell cannot execute it.
    let args = build_attach_args("mac-mini", "default'; rm -rf ~ #");

    assert_eq!(
        args,
        vec![
            "mac-mini",
            "-c",
            "tmux attach -t 'default'\\''; rm -rf ~ #'",
        ]
    );
    // The dangerous fragment must never appear outside the quoted literal.
    assert!(!args[2].contains("default'; rm"));
}

#[test]
fn new_session_args_neutralize_substitution_in_session_names() {
    // Command/back-tick substitution is inert inside single quotes.
    let args = build_new_session_args("mac-mini", "$(touch pwned)`id`");

    assert_eq!(
        args,
        vec![
            "mac-mini",
            "-c",
            "tmux new-session -d -s '$(touch pwned)`id`'",
        ]
    );
}

#[test]
fn attach_args_include_terminal_path_when_known() {
    assert_eq!(
        build_attach_args_with_options(
            "mac-mini",
            Some("devuser"),
            Some("/opt/homebrew/bin/etterminal"),
            "default"
        ),
        vec![
            "mac-mini",
            "--terminal-path",
            "/opt/homebrew/bin/etterminal",
            "-c",
            "tmux attach -t 'default'",
        ]
    );
}

#[test]
fn session_selection_falls_back_to_default_for_missing_or_blank_values() {
    assert_eq!(resolve_session_name(None), "default");
    assert_eq!(resolve_session_name(Some("  ")), "default");
    assert_eq!(resolve_session_name(Some(" pair ")), "pair");
}

#[test]
fn attach_run_uses_client_server_and_default_session() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, "mac-mini");
    let runner = FakeRunner::success();

    run_with(&store, &runner, None, None).unwrap();

    let calls = runner.interactive_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "et");
    assert_eq!(calls[0].1, build_attach_args("mac-mini", "default"));
    assert!(runner.calls.borrow().is_empty());
}

#[test]
fn attach_run_uses_persisted_terminal_path() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_connection(
        &store,
        "mac-mini",
        Some("devuser"),
        Some("/opt/homebrew/bin/etterminal"),
    );
    let runner = FakeRunner::success();

    run_with(&store, &runner, None, None).unwrap();

    let calls = runner.interactive_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "et");
    assert_eq!(
        calls[0].1,
        build_attach_args_with_options(
            "mac-mini",
            Some("devuser"),
            Some("/opt/homebrew/bin/etterminal"),
            "default"
        )
    );
    assert!(runner.calls.borrow().is_empty());
}

#[test]
fn attach_new_creates_session_then_attaches_to_it() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_connection(
        &store,
        "mac-mini",
        Some("devuser"),
        Some("/opt/homebrew/bin/etterminal"),
    );
    let runner = FakeRunner::success();

    run_with(&store, &runner, None, Some("feature")).unwrap();

    let calls = runner.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "et");
    assert_eq!(
        calls[0].1,
        build_new_session_args_with_options(
            "mac-mini",
            Some("devuser"),
            Some("/opt/homebrew/bin/etterminal"),
            "feature"
        )
    );

    let interactive_calls = runner.interactive_calls.borrow();
    assert_eq!(interactive_calls.len(), 1);
    assert_eq!(interactive_calls[0].0, "et");
    assert_eq!(
        interactive_calls[0].1,
        build_attach_args_with_options(
            "mac-mini",
            Some("devuser"),
            Some("/opt/homebrew/bin/etterminal"),
            "feature"
        )
    );
}

#[test]
fn attach_new_does_not_attach_when_session_creation_fails() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, "mac-mini");
    let runner = FakeRunner::failure("duplicate session");

    let error = run_with(&store, &runner, None, Some("feature")).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("command failed: et"));
    assert!(message.contains("stderr: duplicate session"));
    assert!(runner.interactive_calls.borrow().is_empty());
}

#[test]
fn attach_new_error_includes_stdout_when_stderr_is_empty() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, "mac-mini");
    let runner = FakeRunner::failure_with_streams("et could not reach mac-mini", "");

    let error = run_with(&store, &runner, None, Some("feature")).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("command failed: et"));
    assert!(message.contains("stdout: et could not reach mac-mini"));
    assert!(runner.interactive_calls.borrow().is_empty());
}

#[test]
fn attach_new_rejects_existing_session_argument() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, "mac-mini");
    let runner = FakeRunner::success();

    let error = run_with(&store, &runner, Some("default"), Some("feature")).unwrap_err();

    assert!(error
        .to_string()
        .contains("attach accepts either an existing session or --new"));
    assert!(runner.calls.borrow().is_empty());
    assert!(runner.interactive_calls.borrow().is_empty());
}

#[test]
fn attach_run_returns_clear_error_on_non_zero_exit() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, "mac-mini");
    let runner = FakeRunner::failure("unable to connect");

    let error = run_with(&store, &runner, Some("pair"), None).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("command failed: et"));
    assert!(message.contains("stderr: unable to connect"));
}
