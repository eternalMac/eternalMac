use std::io::ErrorKind;

use anyhow::Result;

use crate::config::store::Store;
use crate::model::config::{Config, Role};
use crate::process::runner::Runner;
use crate::status::service::daemon_heartbeat_stale;
use crate::tooling::ping::alive_check_args;

const CONFIG_MISSING_ISSUE: &str =
    "config missing: run `eternalMac setup server` or `eternalMac setup client`";

pub fn collect_issues<R: Runner>(store: &Store, runner: &R) -> Result<Vec<String>> {
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
        inspect_state(store, config, runner, &mut issues)?;
    }

    Ok(issues)
}

fn inspect_state<R: Runner>(
    store: &Store,
    config: &Config,
    runner: &R,
    issues: &mut Vec<String>,
) -> Result<()> {
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

            let issue_count_before_role_health = issues.len();
            if !state.daemon_healthy {
                issues.push(format!("daemon unhealthy: {}", state.summary));
            }

            if matches!(config.role, Role::Client) {
                if !state.tailscale_ok {
                    issues.push("tailscale not running".to_string());
                }
                if !state.server_reachable {
                    match config.client.as_ref() {
                        Some(client) => {
                            let paired_server = client.paired_server.as_str();
                            issues.push(format!(
                                "paired server unreachable over Eternal Terminal: {paired_server}"
                            ));
                            issues.push(server_alive_issue(runner, paired_server));
                        }
                        None => issues.push(
                            "paired server unreachable over Eternal Terminal: unknown".to_string(),
                        ),
                    }
                }
                for sync in state.syncs.iter().filter(|sync| sync.status != "active") {
                    issues.push(format!("sync {}: {}", sync.status, sync.name));
                }
            }

            if !state.healthy && issues.len() == issue_count_before_role_health {
                issues.push(format!("daemon degraded: {}", state.summary));
            }
        }
        Err(error) if is_not_found(&error) => issues.push(state_missing_issue(config)),
        Err(error) => issues.push(format!("state unreadable: {error}")),
    }

    Ok(())
}

// macOS ping(8) exit codes: 0 = at least one reply, 2 = no replies; anything
// else (e.g. 68 for an unresolvable host) means the probe itself failed.
const PING_NO_REPLIES_EXIT_CODE: i32 = 2;

fn server_alive_issue<R: Runner>(runner: &R, paired_server: &str) -> String {
    match runner.run("ping", &alive_check_args(paired_server)) {
        Ok(output) if output.success => format!(
            "server alive: yes ({paired_server} responds to ping; the Eternal Terminal service may be down on the server — try `brew services restart et` there)"
        ),
        Ok(output) if output.exit_code == Some(PING_NO_REPLIES_EXIT_CODE) => format!(
            "server alive: no ({paired_server} did not respond to ping; the server may be offline or asleep)"
        ),
        Ok(output) => {
            let detail = match output.exit_code {
                Some(code) => format!("ping exited with code {code}"),
                None => "ping terminated without an exit code".to_string(),
            };
            let stderr = output.stderr.trim();
            if stderr.is_empty() {
                format!("server alive: unknown ({detail})")
            } else {
                format!("server alive: unknown ({detail}: {stderr})")
            }
        }
        Err(error) => format!("server alive: unknown (could not run ping: {error})"),
    }
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
