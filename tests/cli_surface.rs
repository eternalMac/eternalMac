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
        .stdout(
            contains("Usage: eternalMac")
                .and(contains("attach"))
                .and(contains("session")),
        )
        .stdout(predicates::str::contains("daemon").not());
}

#[test]
fn subcommand_help_is_descriptive() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["attach", "--help"])
        .assert()
        .success()
        .stdout(contains("Attach to a session").and(contains("-n, --new <SESSION>")));

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
        .stdout(
            contains("Add a sync pair")
                .and(contains("--local <LOCAL>"))
                .and(contains("--remote <REMOTE>")),
        );

    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["daemon", "server", "--help"])
        .assert()
        .success()
        .stdout(contains("Start the daemon server"));
}

#[test]
fn setup_help_mentions_server_and_client_targets() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["setup", "--help"])
        .assert()
        .success()
        .stdout(
            contains("Commands:")
                .and(contains(
                    "server  Configure this machine as the setup server",
                ))
                .and(contains("client  Configure this machine as a setup client")),
        );
}

#[test]
fn setup_client_help_keeps_server_override_optional() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["setup", "client", "--help"])
        .assert()
        .success()
        .stdout(
            contains("Usage: eternalMac setup client [OPTIONS]").and(contains(
                "--server <SERVER>  Override the server DNS name to pair with",
            )),
        );
}

#[test]
fn hidden_daemon_server_is_invocable() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["daemon", "--help"])
        .assert()
        .success()
        .stdout(contains("Run daemon commands").and(contains("server")));
}
