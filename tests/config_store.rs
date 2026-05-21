use eternalmac::app::paths::Paths;
use eternalmac::config::store::Store;
use eternalmac::model::config::{
    ClientConfig, Config, Role, ServerConfig, SessionConfig, SyncPairConfig,
};
use eternalmac::model::state::State;

#[test]
fn config_round_trip_preserves_server_dns_and_client_sync_pairs() {
    let tempdir = tempfile::tempdir().unwrap();
    let paths = Paths::new(tempdir.path().to_path_buf());
    let store = Store::new(paths.clone());

    let config = Config {
        role: Role::Server,
        server: Some(ServerConfig {
            host_label: "mac-mini".into(),
            default_session: "default".into(),
            boot_sessions: vec!["default".into()],
            tailscale_dns: Some("eternalmac.local".into()),
        }),
        client: Some(ClientConfig {
            paired_server: "server-1".into(),
            server_ssh_user: Some("kindshadow".into()),
            server_etterminal_path: Some("/opt/homebrew/bin/etterminal".into()),
            pinned: vec!["workspace".into()],
            sync_pairs: vec![SyncPairConfig {
                name: "docs".into(),
                local: "/Users/me/Documents".into(),
                remote: "/Volumes/docs".into(),
                mode: "mirror".into(),
            }],
        }),
        session: SessionConfig { auto_attach: true },
    };

    store.save_config(&config).unwrap();
    let loaded = store.load_config().unwrap();

    assert!(matches!(loaded.role, Role::Server));
    assert_eq!(loaded.server.as_ref().unwrap().default_session, "default");
    assert_eq!(
        loaded
            .server
            .as_ref()
            .and_then(|server| server.tailscale_dns.clone()),
        Some("eternalmac.local".into())
    );
    assert_eq!(loaded.client.as_ref().unwrap().sync_pairs.len(), 1);
    assert_eq!(
        loaded.client.as_ref().unwrap().server_ssh_user.as_deref(),
        Some("kindshadow")
    );
    assert_eq!(
        loaded
            .client
            .as_ref()
            .unwrap()
            .server_etterminal_path
            .as_deref(),
        Some("/opt/homebrew/bin/etterminal")
    );
    assert_eq!(loaded.client.as_ref().unwrap().sync_pairs[0].name, "docs");
    assert_eq!(
        paths.config_file,
        tempdir.path().join(".config/eternalmac/config.toml")
    );
    assert_eq!(
        paths.launch_agents_dir,
        tempdir.path().join("Library/LaunchAgents")
    );
    assert_eq!(
        paths.server_plist,
        tempdir
            .path()
            .join("Library/LaunchAgents/com.eternalmac.server.plist")
    );
    assert_eq!(
        paths.client_plist,
        tempdir
            .path()
            .join("Library/LaunchAgents/com.eternalmac.client.plist")
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
        tailscale_dns: Some("eternalmac.local".into()),
        daemon_healthy: true,
        daemon_heartbeat_unix: 1_714_000_000,
        default_session_present: true,
        known_sessions: vec!["default".into(), "sync".into()],
        syncs: vec![eternalmac::model::state::SyncPairState {
            name: "docs".into(),
            local: "/Users/me/Documents".into(),
            remote: "/Volumes/docs".into(),
            mode: "mirror".into(),
            status: "synced".into(),
        }],
    };

    store.save_state(&state).unwrap();
    let loaded = store.load_state().unwrap();

    assert!(paths.state_file.exists());
    assert!(!paths.config_file.exists());
    assert_eq!(loaded.daemon_heartbeat_unix, 1_714_000_000);
    assert_eq!(loaded.known_sessions, vec!["default", "sync"]);
    assert_eq!(loaded.syncs[0].status, "synced");
}
