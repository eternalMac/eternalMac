use std::fs;
use std::io::ErrorKind;

use anyhow::{anyhow, Context, Result};

use crate::app::paths::Paths;
use crate::config::store::Store;
use crate::model::config::{Config, Role, ServerConfig, SessionConfig};
use crate::model::state::State;
use crate::platform::launchd::{write_plist, Definition};
use crate::process::runner::{Output, Runner};
use crate::tooling::brew::{install_cask_args, install_formula_args};
use crate::tooling::tailscale::{parse_status_json, status_args, Status as TailscaleStatus};
use crate::tooling::tmux::{list_sessions_args, new_session_args, parse_sessions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSetupSummary {
    pub dns_name: String,
    pub default_session: String,
}

fn run_checked<R: Runner>(runner: &R, program: &str, args: &[String]) -> Result<Output> {
    let output = runner.run(program, args)?;
    if output.success {
        return Ok(output);
    }

    let mut message = format!("command failed: {program} {}", args.join(" "));
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }

    Err(anyhow!(message))
}

fn is_expected_unload_absence(output: &Output) -> bool {
    let combined = format!(
        "{}\n{}",
        output.stdout.to_lowercase(),
        output.stderr.to_lowercase()
    );

    combined.contains("could not find specified service")
        || combined.contains("could not find service")
        || combined.contains("not loaded")
        || combined.contains("no such process")
        || combined.contains("not found")
}

fn run_unload_checked<R: Runner>(runner: &R, plist_path: &std::path::Path) -> Result<()> {
    let args = vec![
        "unload".into(),
        "-w".into(),
        plist_path.display().to_string(),
    ];
    let output = runner.run("launchctl", &args)?;
    if output.success || is_expected_unload_absence(&output) {
        return Ok(());
    }

    let mut message = format!("command failed: launchctl {}", args.join(" "));
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }
    if !output.stdout.trim().is_empty() {
        message.push_str(&format!("; stdout: {}", output.stdout.trim()));
    }

    Err(anyhow!(message))
}

fn persist_config_and_state(
    paths: &Paths,
    store: &Store,
    config: &Config,
    state: &State,
) -> Result<()> {
    let prior_config = match fs::read(&paths.config_file) {
        Ok(previous) => Some(previous),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(anyhow!(
                "failed to snapshot existing config before setup write: {error}"
            ));
        }
    };
    store.save_config(config)?;

    if let Err(state_error) = store.save_state(state) {
        let rollback = match prior_config {
            Some(previous) => fs::write(&paths.config_file, previous),
            None => match fs::remove_file(&paths.config_file) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            },
        };

        return match rollback {
            Ok(()) => Err(anyhow!(
                "failed to persist setup state after config write: {state_error}"
            )),
            Err(rollback_error) => Err(anyhow!(
                "failed to persist setup state after config write: {state_error}; config rollback failed: {rollback_error}"
            )),
        };
    }

    Ok(())
}

fn resolve_server_dns(status: &TailscaleStatus) -> Result<String> {
    if status.backend_state != "Running" {
        return Err(anyhow!(
            "tailscale is not connected (backend state: {}); sign in to Tailscale and rerun `eternalMac setup server`",
            status.backend_state
        ));
    }

    status.dns_name.clone().ok_or_else(|| {
        anyhow!(
            "tailscale dns name is unavailable; finish Tailscale login and rerun `eternalMac setup server`"
        )
    })
}

fn current_executable_path() -> Result<String> {
    Ok(std::env::current_exe()
        .context("resolving current executable path for server launch agent")?
        .display()
        .to_string())
}

fn tmux_list_sessions_failed_without_server(stderr: &str) -> bool {
    let normalized = stderr.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    normalized.contains("no server running on")
        || normalized.contains("failed to connect to server")
        || (normalized.contains("error connecting to")
            && (normalized.contains("no such file or directory")
                || normalized.contains("connection refused")))
}

pub fn apply_server_setup<R: Runner>(
    paths: &Paths,
    store: &Store,
    runner: &R,
    host_label: String,
) -> Result<ServerSetupSummary> {
    let formulae = vec!["et".into(), "tmux".into(), "mutagen".into()];
    if let Some(args) = install_formula_args(&formulae) {
        run_checked(runner, "brew", &args)?;
    }

    let cask_args = install_cask_args("tailscale-app");
    run_checked(runner, "brew", &cask_args)?;

    let tailscale_args = status_args();
    let tailscale_status = run_checked(runner, "tailscale", &tailscale_args)?;
    let parsed_status = parse_status_json(&tailscale_status.stdout)?;
    let dns_name = resolve_server_dns(&parsed_status)?;
    let tailscale_ok = true;

    let default_session = "default".to_string();

    let config = Config {
        role: Role::Server,
        server: Some(ServerConfig {
            host_label,
            default_session: default_session.clone(),
            boot_sessions: vec![default_session.clone()],
            tailscale_dns: Some(dns_name.clone()),
        }),
        client: None,
        session: SessionConfig { auto_attach: true },
    };
    persist_config_and_state(
        paths,
        store,
        &config,
        &State {
            role: Role::Server,
            tailscale_ok,
            server_reachable: false,
            healthy: false,
            summary: "server setup in progress; finalizing runtime prerequisites".into(),
            tailscale_dns: Some(dns_name.clone()),
            daemon_healthy: false,
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: vec![],
        },
    )?;

    let tmux_list_args = list_sessions_args();
    let tmux_list_output = runner.run("tmux", &tmux_list_args)?;
    let mut known_sessions = if tmux_list_output.success {
        parse_sessions(&tmux_list_output.stdout)
    } else if tmux_list_sessions_failed_without_server(&tmux_list_output.stderr) {
        vec![]
    } else {
        let mut message = format!("command failed: tmux {}", tmux_list_args.join(" "));
        if !tmux_list_output.stderr.trim().is_empty() {
            message.push_str(&format!("; stderr: {}", tmux_list_output.stderr.trim()));
        }
        return Err(anyhow!(message));
    };

    if !known_sessions.iter().any(|session| session == &default_session) {
        let tmux_args = new_session_args(&default_session);
        run_checked(runner, "tmux", &tmux_args)?;
        known_sessions.push(default_session.clone());
    }
    let executable_path = current_executable_path()?;

    write_plist(
        &paths.server_plist,
        &Definition {
            label: "com.eternalmac.server".into(),
            program_arguments: vec![
                executable_path,
                "daemon".into(),
                "server".into(),
            ],
            run_at_load: true,
            keep_alive: true,
        },
    )?;

    run_unload_checked(runner, &paths.client_plist)?;

    let launchctl_args = vec![
        "load".into(),
        "-w".into(),
        paths.server_plist.display().to_string(),
    ];
    run_checked(runner, "launchctl", &launchctl_args)?;

    let server_reachable = true;
    let healthy = tailscale_ok && server_reachable;
    store.save_state(&State {
        role: Role::Server,
        tailscale_ok,
        server_reachable,
        healthy,
        summary: "server setup complete; runtime health pending".into(),
        tailscale_dns: Some(dns_name.clone()),
        daemon_healthy: false,
        daemon_heartbeat_unix: 0,
        default_session_present: true,
        known_sessions,
        syncs: vec![],
    })?;

    Ok(ServerSetupSummary {
        dns_name,
        default_session,
    })
}
