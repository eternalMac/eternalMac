use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn doctor_command_exists() {
    Command::cargo_bin("eternalMac")
        .unwrap()
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains("no issues"));
}
