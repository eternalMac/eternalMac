use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::model::config::{Config, Role, ServerConfig, SessionConfig};
use eternalmac::model::state::State;

#[test]
fn config_round_trip_preserves_role_and_default_session() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());

    let config = Config {
        role: Role::Server,
        server: Some(ServerConfig {
            host_label: "mac-mini".into(),
            default_session: "default".into(),
            boot_sessions: vec!["default".into()],
        }),
        client: None,
        session: SessionConfig { auto_attach: true },
    };

    store.save_config(&config).unwrap();
    let loaded = store.load_config().unwrap();

    assert!(matches!(loaded.role, Role::Server));
    assert_eq!(loaded.server.as_ref().unwrap().default_session, "default");
    assert!(loaded.client.is_none());
    assert_eq!(
        paths.config_file,
        tempdir.path().join(".config/eternalmac/config.toml")
    );
    assert_eq!(
        paths.state_file,
        tempdir
            .path()
            .join("Library/Application Support/eternalmac/state.json")
    );
    assert!(paths.config_file.exists());
    assert!(!paths.state_file.exists());
}

#[test]
fn save_state_writes_state_file_to_state_location() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());

    let state = State {
        role: Role::Client,
        tailscale_ok: true,
        server_reachable: true,
        healthy: true,
        summary: "ok".into(),
    };

    store.save_state(&state).unwrap();

    assert!(paths.state_file.exists());
    assert!(!paths.config_file.exists());
}
