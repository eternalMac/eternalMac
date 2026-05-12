use std::io::ErrorKind;

use anyhow::Result;

use crate::config::store::Store;
use crate::model::config::{Config, Role};
use crate::status::service::daemon_heartbeat_stale;

const CONFIG_MISSING_ISSUE: &str =
    "config missing: run `eternalMac setup server` or `eternalMac setup client`";

pub fn collect_issues(store: &Store) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    let config = match store.load_config() {
        Ok(config) => Some(config),
        Err(error) if is_not_found(&error) => {
            issues.push(CONFIG_MISSING_ISSUE.to_string());
            None
        }
        Err(error) => {
            issues.push(format!("config unreadable: {error}"));
            None
        }
    };

    if let Some(config) = config.as_ref() {
        inspect_state(store, config, &mut issues)?;
    }

    Ok(issues)
}

fn inspect_state(store: &Store, config: &Config, issues: &mut Vec<String>) -> Result<()> {
    match store.load_state() {
        Ok(state) => {
            if role_name(&state.role) != role_name(&config.role) {
                issues.push(format!(
                    "role mismatch: config is {}, state is {}",
                    role_name(&config.role),
                    role_name(&state.role)
                ));
            }

            if daemon_heartbeat_stale(&state)? {
                issues.push(
                    "daemon heartbeat stale: daemon has not refreshed local health recently"
                        .to_string(),
                );
            }
        }
        Err(error) if is_not_found(&error) => issues.push(state_missing_issue(config)),
        Err(error) => issues.push(format!("state unreadable: {error}")),
    }

    Ok(())
}

fn state_missing_issue(config: &Config) -> String {
    match config.role {
        Role::Server => {
            "state missing: re-run `eternalMac setup server` to restore local state".to_string()
        }
        Role::Client => {
            "state missing: re-run `eternalMac setup client` to restore local state".to_string()
        }
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Server => "server",
        Role::Client => "client",
    }
}

fn is_not_found(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        if let Some(io_error) = cause.downcast_ref::<std::io::Error>() {
            return io_error.kind() == ErrorKind::NotFound;
        }

        false
    })
}
