use anyhow::{anyhow, Result};

use crate::app::paths::Paths;
use crate::config::store::Store;
use crate::model::config::{ClientConfig, Config, Role, SessionConfig, SyncPairConfig};
use crate::model::state::{State, SyncPairState};
use crate::platform::launchd::{write_plist, Definition};
use crate::process::runner::{Output, Runner};
use crate::tooling::brew::{install_cask_args, install_formula_args};
use crate::tooling::mutagen::{build_create_args, SYNC_MODE_TWO_WAY_RESOLVED};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRootInput {
    pub name: String,
    pub local: String,
    pub remote: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSetupInput {
    pub paired_server: String,
    pub sync_roots: Vec<SyncRootInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSetupSummary {
    pub paired_server: String,
    pub sync_names: Vec<String>,
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

pub fn apply_client_setup<R: Runner>(
    paths: &Paths,
    store: &Store,
    runner: &R,
    input: ClientSetupInput,
) -> Result<ClientSetupSummary> {
    let formulae = vec!["et".into(), "tmux".into(), "mutagen".into()];
    if let Some(args) = install_formula_args(&formulae) {
        run_checked(runner, "brew", &args)?;
    }

    let cask_args = install_cask_args("tailscale-app");
    run_checked(runner, "brew", &cask_args)?;

    let sync_pairs = input
        .sync_roots
        .iter()
        .map(|root| SyncPairConfig {
            name: root.name.clone(),
            local: root.local.clone(),
            remote: root.remote.clone(),
            mode: SYNC_MODE_TWO_WAY_RESOLVED.into(),
        })
        .collect::<Vec<_>>();

    for root in &input.sync_roots {
        let args = build_create_args(&root.name, &root.local, &root.remote);
        run_checked(runner, "mutagen", &args)?;
    }

    let sync_names = sync_pairs
        .iter()
        .map(|pair| pair.name.clone())
        .collect::<Vec<_>>();

    write_plist(
        &paths.client_plist,
        &Definition {
            label: "com.eternalmac.client".into(),
            program_arguments: vec![
                "/opt/homebrew/bin/eternalMac".into(),
                "daemon".into(),
                "client".into(),
            ],
            run_at_load: true,
            keep_alive: true,
        },
    )?;

    let launchctl_args = vec![
        "load".into(),
        "-w".into(),
        paths.client_plist.display().to_string(),
    ];
    run_checked(runner, "launchctl", &launchctl_args)?;

    store.save_config(&Config {
        role: Role::Client,
        server: None,
        client: Some(ClientConfig {
            paired_server: input.paired_server.clone(),
            pinned: vec![],
            sync_pairs: sync_pairs.clone(),
        }),
        session: SessionConfig { auto_attach: true },
    })?;

    store.save_state(&State {
        role: Role::Client,
        tailscale_ok: false,
        server_reachable: false,
        healthy: false,
        summary: "client setup complete; runtime health pending".into(),
        tailscale_dns: None,
        daemon_healthy: false,
        daemon_heartbeat_unix: 0,
        default_session_present: false,
        known_sessions: vec![],
        syncs: sync_pairs
            .iter()
            .map(|pair| SyncPairState {
                name: pair.name.clone(),
                local: pair.local.clone(),
                remote: pair.remote.clone(),
                mode: pair.mode.clone(),
                status: "created".into(),
            })
            .collect(),
    })?;

    Ok(ClientSetupSummary {
        paired_server: input.paired_server,
        sync_names,
    })
}
