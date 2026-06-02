use std::cell::RefCell;

use eternalmac::app::paths::Paths;
use eternalmac::commands::sync::{add_with, add_with_ignores, list_with, status_with};
use eternalmac::config::store::Store;
use eternalmac::model::config::{ClientConfig, Config, Role, SessionConfig, SyncPairConfig};
use eternalmac::process::runner::{Output, Runner};
use eternalmac::sync::service::build_pair;
use eternalmac::tooling::mutagen::{
    build_create_args, build_create_args_with_ignores, list_args, SYNC_MODE_TWO_WAY_RESOLVED,
};

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
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<Output> {
        self.calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));
        Ok(self.output.clone())
    }
}

fn save_client_config(store: &Store, sync_pairs: Vec<SyncPairConfig>) {
    save_client_config_with_server_user(store, None, sync_pairs);
}

fn save_client_config_with_server_user(
    store: &Store,
    server_ssh_user: Option<&str>,
    sync_pairs: Vec<SyncPairConfig>,
) {
    store
        .save_config(&Config {
            role: Role::Client,
            server: None,
            client: Some(ClientConfig {
                paired_server: "mac-mini".into(),
                server_ssh_user: server_ssh_user.map(str::to_string),
                server_etterminal_path: None,
                pinned: vec![],
                sync_pairs,
            }),
            session: SessionConfig { auto_attach: true },
        })
        .unwrap();
}

#[test]
fn sync_pair_uses_default_mutagen_mode_when_unspecified() {
    let pair = build_pair("project", "~/src/project", "~/remote/project", None);
    assert_eq!(pair.name, "project");
    assert_eq!(pair.mode, SYNC_MODE_TWO_WAY_RESOLVED);
}

#[test]
fn sync_pair_normalizes_mode_case_and_separator() {
    let pair = build_pair(
        "project",
        "~/src/project",
        "~/remote/project",
        Some(" TWO_WAY_RESOLVED "),
    );
    assert_eq!(pair.mode, SYNC_MODE_TWO_WAY_RESOLVED);
}

#[test]
fn sync_pair_falls_back_to_default_mode_for_unknown_mode() {
    let pair = build_pair(
        "project",
        "~/src/project",
        "~/remote/project",
        Some("one-way-safe"),
    );
    assert_eq!(pair.mode, SYNC_MODE_TWO_WAY_RESOLVED);
}

#[test]
fn mutagen_create_args_include_sync_mode_in_order() {
    let args = build_create_args("project", "~/src/project", "~/remote/project");
    assert_eq!(
        args,
        vec![
            "sync",
            "create",
            "--name",
            "project",
            "--sync-mode",
            "two-way-resolved",
            "~/src/project",
            "~/remote/project",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>()
    );
}

#[test]
fn mutagen_create_args_with_ignores_insert_repeated_ignore_flags_before_endpoints() {
    let args = build_create_args_with_ignores(
        "dotsync-claude",
        "/Users/me/.claude",
        "devuser@mac-mini:~/.claude",
        &[".DS_Store".to_string(), ".claude.json".to_string()],
    );

    assert_eq!(
        args,
        vec![
            "sync",
            "create",
            "--name",
            "dotsync-claude",
            "--sync-mode",
            "two-way-resolved",
            "--ignore",
            ".DS_Store",
            "--ignore",
            ".claude.json",
            "/Users/me/.claude",
            "devuser@mac-mini:~/.claude",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>()
    );
}

#[test]
fn sync_pair_uses_the_same_mode_as_mutagen_create_args() {
    let pair = build_pair("project", "~/src/project", "~/remote/project", None);
    let args = build_create_args("project", "~/src/project", "~/remote/project");

    assert_eq!(pair.mode, SYNC_MODE_TWO_WAY_RESOLVED);
    assert!(args.iter().any(|value| value == SYNC_MODE_TWO_WAY_RESOLVED));
}

#[test]
fn sync_add_runs_mutagen_create_and_persists_pair() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, vec![]);
    let runner = FakeRunner::success("");

    let sync_pair = add_with(
        &store,
        &runner,
        "project",
        "~/src/project",
        "~/remote/project",
    )
    .unwrap();

    assert_eq!(sync_pair.mode, SYNC_MODE_TWO_WAY_RESOLVED);
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "mutagen".into(),
            build_create_args("project", "~/src/project", "mac-mini:~/remote/project")
        )]
    );
    let config = store.load_config().unwrap();
    let saved_pairs = &config.client.unwrap().sync_pairs;
    assert_eq!(saved_pairs.len(), 1);
    let saved = &saved_pairs[0];
    assert_eq!(saved.name, sync_pair.name);
    assert_eq!(saved.local, sync_pair.local);
    assert_eq!(saved.remote, sync_pair.remote);
    assert_eq!(saved.mode, sync_pair.mode);
}

#[test]
fn sync_add_accepts_explicit_ignore_patterns_and_persists_them() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_server_user(&store, Some("devuser"), vec![]);
    let runner = FakeRunner::success("");

    let sync_pair = add_with_ignores(
        &store,
        &runner,
        "project",
        "~/src/project",
        "~/remote/project",
        &[".env".to_string(), "secrets/".to_string()],
    )
    .unwrap();

    assert_eq!(sync_pair.ignore_paths, vec![".env", "secrets/"]);
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "mutagen".into(),
            build_create_args_with_ignores(
                "project",
                "~/src/project",
                "devuser@mac-mini:~/remote/project",
                &[".env".to_string(), "secrets/".to_string()]
            )
        )]
    );
    let config = store.load_config().unwrap();
    let saved_pairs = config.client.unwrap().sync_pairs;
    assert_eq!(saved_pairs[0].ignore_paths, vec![".env", "secrets/"]);
}

#[test]
fn sync_add_resolves_bare_remote_path_against_paired_server() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_server_user(&store, Some("devuser"), vec![]);
    let runner = FakeRunner::success("");

    let sync_pair = add_with(
        &store,
        &runner,
        "project",
        "~/src/project",
        "~/remote/project",
    )
    .unwrap();

    assert_eq!(sync_pair.remote, "devuser@mac-mini:~/remote/project");
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "mutagen".into(),
            build_create_args(
                "project",
                "~/src/project",
                "devuser@mac-mini:~/remote/project"
            )
        )]
    );
    let config = store.load_config().unwrap();
    let saved_pairs = config.client.unwrap().sync_pairs;
    assert_eq!(saved_pairs[0].remote, "devuser@mac-mini:~/remote/project");
}

#[test]
fn sync_add_preserves_full_remote_endpoint() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config_with_server_user(&store, Some("devuser"), vec![]);
    let runner = FakeRunner::success("");

    let sync_pair = add_with(
        &store,
        &runner,
        "project",
        "~/src/project",
        "other@devbox:~/remote/project",
    )
    .unwrap();

    assert_eq!(sync_pair.remote, "other@devbox:~/remote/project");
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        &[(
            "mutagen".into(),
            build_create_args("project", "~/src/project", "other@devbox:~/remote/project")
        )]
    );
}

#[test]
fn sync_add_returns_clear_error_on_non_zero_exit_without_persisting_pair() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(&store, vec![]);
    let runner = FakeRunner::failure("daemon unavailable");

    let error = add_with(
        &store,
        &runner,
        "project",
        "~/src/project",
        "~/remote/project",
    )
    .unwrap_err();

    assert!(error.to_string().contains("command failed: mutagen"));
    assert!(error.to_string().contains("stderr: daemon unavailable"));
    let config = store.load_config().unwrap();
    let saved_pairs = config.client.unwrap().sync_pairs;
    assert!(saved_pairs.is_empty());
}

#[test]
fn sync_add_rejects_reusing_a_name_for_different_endpoints() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(
        &store,
        vec![SyncPairConfig {
            name: "project".into(),
            local: "~/src/project".into(),
            remote: "~/remote/project".into(),
            mode: SYNC_MODE_TWO_WAY_RESOLVED.into(),
            ignore_paths: vec![],
            kind: None,
            label: None,
        }],
    );
    let runner = FakeRunner::success("");

    let error = add_with(
        &store,
        &runner,
        "project",
        "~/src/other-project",
        "~/remote/other-project",
    )
    .unwrap_err();

    assert!(error.to_string().contains("different endpoints"));
    let config = store.load_config().unwrap();
    let saved_pairs = config.client.unwrap().sync_pairs;
    assert_eq!(saved_pairs.len(), 1);
    assert_eq!(saved_pairs[0].local, "~/src/project");
    assert_eq!(saved_pairs[0].remote, "~/remote/project");

    let calls = runner.calls.borrow();
    assert!(calls.is_empty());
}

#[test]
fn sync_list_reads_persisted_sync_pairs_from_config() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
    save_client_config(
        &store,
        vec![SyncPairConfig {
            name: "project".into(),
            local: "~/src/project".into(),
            remote: "~/remote/project".into(),
            mode: SYNC_MODE_TWO_WAY_RESOLVED.into(),
            ignore_paths: vec![],
            kind: None,
            label: None,
        }],
    );

    let pairs = list_with(&store).unwrap();

    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].name, "project");
    assert_eq!(pairs[0].local, "~/src/project");
    assert_eq!(pairs[0].remote, "~/remote/project");
    assert_eq!(pairs[0].mode, SYNC_MODE_TWO_WAY_RESOLVED);
}

#[test]
fn sync_status_runs_mutagen_list_and_returns_output() {
    let runner = FakeRunner::success("Status: Watching for changes");

    let output = status_with(&runner).unwrap();

    assert_eq!(output, "Status: Watching for changes");
    let calls = runner.calls.borrow();
    assert_eq!(calls.as_slice(), &[("mutagen".into(), list_args())]);
}
