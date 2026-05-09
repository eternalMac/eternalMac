use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn top_level_help_hides_daemon_command() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage: eternalMac").and(contains("attach")).and(contains("session")))
        .stdout(predicates::str::contains("daemon").not());
}

#[test]
fn subcommand_help_is_descriptive() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["attach", "--help"])
        .assert()
        .success()
        .stdout(contains("Attach to a session"));

    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["session", "--help"])
        .assert()
        .success()
        .stdout(contains("Manage sessions"));

    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(contains("Manage sync pairs"));

    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["session", "new", "--help"])
        .assert()
        .success()
        .stdout(contains("Create a new session"));

    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["sync", "add", "--help"])
        .assert()
        .success()
        .stdout(contains("Add a sync pair"));

    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["daemon", "server", "--help"])
        .assert()
        .success()
        .stdout(contains("Start the daemon server"));
}

#[test]
fn attach_routes_to_existing_output() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["attach"])
        .assert()
        .success()
        .stdout(contains("mac-mini").and(contains("tmux attach -t 'default'")));
}

#[test]
fn session_new_routes_correctly() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["session", "new", "demo"])
        .assert()
        .success()
        .stdout(contains("created demo"));
}

#[test]
fn sync_add_routes_and_respects_long_flags() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args([
            "sync",
            "add",
            "project",
            "--local",
            "~/src/project",
            "--remote",
            "~/remote/project",
        ])
        .assert()
        .success()
        .stdout(contains("sync project ~/src/project ~/remote/project"));
}

#[test]
fn hidden_daemon_server_is_invocable() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["daemon", "server"])
        .assert()
        .success()
        .stdout(contains("server reconciler tick"));
}
