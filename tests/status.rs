use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn status_command_exists() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["status"])
        .assert()
        .success()
        .stdout(contains("healthy"));
}
