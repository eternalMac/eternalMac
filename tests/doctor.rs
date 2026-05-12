use assert_cmd::Command;
use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::model::config::{Config, Role, ServerConfig, SessionConfig};
use predicates::str::contains;

#[test]
fn doctor_reports_missing_config_on_unconfigured_machine() {
    let tempdir = tempfile::tempdir().unwrap();

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains("config missing: run `eternalMac setup server` or `eternalMac setup client`"));
}

#[test]
fn doctor_reports_missing_state_when_config_exists() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths);
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

    Command::cargo_bin("eternalMac")
        .unwrap()
        .env("HOME", tempdir.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(contains("state missing: re-run `eternalMac setup server` to restore local state"));
}
