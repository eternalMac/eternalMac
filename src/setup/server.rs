use anyhow::{anyhow, Result};

use crate::app::paths::Paths;
use crate::config::store::Store;
use crate::model::config::{Config, Role, ServerConfig, SessionConfig};
use crate::model::state::State;
use crate::platform::launchd::{write_plist, Definition};
use crate::process::runner::{Output, Runner};
use crate::tooling::brew::{install_cask_args, install_formula_args};
use crate::tooling::tailscale::{parse_status_json, status_args};
use crate::tooling::tmux::new_session_args;

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
    let dns_name = parsed_status
        .dns_name
        .unwrap_or_else(|| format!("{host_label}.unknown.ts.net"));
    let tailscale_ok = parsed_status.backend_state == "Running";

    let default_session = "default".to_string();
    write_plist(
        &paths.server_plist,
        &Definition {
            label: "com.eternalmac.server".into(),
            program_arguments: vec![
                "/opt/homebrew/bin/eternalMac".into(),
                "daemon".into(),
                "server".into(),
            ],
            run_at_load: true,
            keep_alive: true,
        },
    )?;

    let launchctl_args = vec![
        "load".into(),
        "-w".into(),
        paths.server_plist.display().to_string(),
    ];
    run_checked(runner, "launchctl", &launchctl_args)?;

    let tmux_args = new_session_args(&default_session);
    run_checked(runner, "tmux", &tmux_args)?;

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
    store.save_config(&config)?;

    store.save_state(&State {
        role: Role::Server,
        tailscale_ok,
        server_reachable: false,
        healthy: false,
        summary: "server setup complete; runtime health pending".into(),
        tailscale_dns: Some(dns_name.clone()),
        daemon_healthy: false,
        daemon_heartbeat_unix: 0,
        default_session_present: true,
        known_sessions: vec![default_session.clone()],
        syncs: vec![],
    })?;

    Ok(ServerSetupSummary {
        dns_name,
        default_session,
    })
}
