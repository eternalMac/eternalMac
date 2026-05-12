use std::io::ErrorKind;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::config::store::Store;
use crate::model::config::{Config, Role};
use crate::model::state::State;

#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    pub config: Config,
    pub state: State,
}

#[derive(Debug, Clone)]
pub enum StatusLoad {
    Ready(StatusSnapshot),
    ConfigMissing,
    StateMissing { config: Config },
    RoleMismatch { config_role: Role, state_role: Role },
}

const DAEMON_HEARTBEAT_STALE_AFTER_SECONDS: i64 = 90;

pub fn load(store: &Store) -> Result<StatusLoad> {
    let config = match store.load_config() {
        Ok(config) => config,
        Err(error) if is_not_found(&error) => return Ok(StatusLoad::ConfigMissing),
        Err(error) => return Err(error),
    };

    let state = match store.load_state() {
        Ok(state) => state,
        Err(error) if is_not_found(&error) => return Ok(StatusLoad::StateMissing { config }),
        Err(error) => return Err(error),
    };

    if !roles_match(&config.role, &state.role) {
        return Ok(StatusLoad::RoleMismatch {
            config_role: config.role.clone(),
            state_role: state.role.clone(),
        });
    }

    Ok(StatusLoad::Ready(StatusSnapshot { config, state }))
}

pub fn render_summary(snapshot: &StatusSnapshot) -> String {
    let daemon_heartbeat_stale = daemon_heartbeat_stale(&snapshot.state).unwrap_or(false);
    let effective_daemon_healthy = snapshot.state.daemon_healthy && !daemon_heartbeat_stale;
    let effective_healthy = snapshot.state.healthy && !daemon_heartbeat_stale;
    let summary = if daemon_heartbeat_stale {
        format!("{}; daemon heartbeat stale", snapshot.state.summary)
    } else {
        snapshot.state.summary.clone()
    };

    let mut lines = vec![
        format!("role: {}", role_name(&snapshot.config.role)),
        format!("healthy: {}", yes_no(effective_healthy)),
        format!("summary: {}", summary),
        format!("tailscale: {}", yes_no(snapshot.state.tailscale_ok)),
        format!("daemon healthy: {}", yes_no(effective_daemon_healthy)),
        format!(
            "daemon heartbeat stale: {}",
            yes_no(daemon_heartbeat_stale)
        ),
        format!(
            "server reachable: {}",
            yes_no(snapshot.state.server_reachable)
        ),
    ];

    if let Some(dns_name) = snapshot.state.tailscale_dns.as_deref() {
        lines.push(format!("tailscale dns: {dns_name}"));
    }

    match &snapshot.config.role {
        Role::Server => {
            lines.push(format!(
                "default session present: {}",
                yes_no(snapshot.state.default_session_present)
            ));
            lines.push(format!(
                "known sessions: {}",
                comma_or_none(&snapshot.state.known_sessions)
            ));
        }
        Role::Client => {
            if let Some(client_config) = snapshot.config.client.as_ref() {
                lines.push(format!("paired server: {}", client_config.paired_server));
                lines.push(format!(
                    "pinned sessions: {}",
                    comma_or_none(&client_config.pinned)
                ));
            }

            let sync_summary = if snapshot.state.syncs.is_empty() {
                "none".to_string()
            } else {
                snapshot
                    .state
                    .syncs
                    .iter()
                    .map(|sync| format!("{}:{}", sync.name, sync.status))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            lines.push(format!("syncs: {sync_summary}"));
        }
    }

    lines.join("\n")
}

fn comma_or_none(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    values.join(", ")
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "yes"
    } else {
        "no"
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Server => "server",
        Role::Client => "client",
    }
}

fn roles_match(config_role: &Role, state_role: &Role) -> bool {
    matches!(
        (config_role, state_role),
        (Role::Server, Role::Server) | (Role::Client, Role::Client)
    )
}

pub fn daemon_heartbeat_stale(state: &State) -> Result<bool> {
    if !state.daemon_healthy || state.daemon_heartbeat_unix <= 0 {
        return Ok(false);
    }

    let now = current_unix_seconds()?;
    Ok(now - state.daemon_heartbeat_unix > DAEMON_HEARTBEAT_STALE_AFTER_SECONDS)
}

fn current_unix_seconds() -> Result<i64> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    i64::try_from(seconds).context("unix timestamp overflow converting u64 to i64")
}

fn is_not_found(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        if let Some(io_error) = cause.downcast_ref::<std::io::Error>() {
            return io_error.kind() == ErrorKind::NotFound;
        }

        false
    })
}
