use std::fs;
use std::io::ErrorKind;

use anyhow::{anyhow, Context, Result};

use crate::app::paths::Paths;
use crate::config::store::Store;
use crate::model::config::{ClientConfig, Config, Role, SessionConfig, SyncPairConfig};
use crate::model::state::{State, SyncPairState};
use crate::platform::launchd::{write_plist, Definition};
use crate::process::runner::{Output, Runner};
use crate::tooling::brew::{install_cask_args, install_formula_args};
use crate::tooling::mutagen::{
    build_create_args, list_args as mutagen_list_args, parse_list_output, ListedSession,
    SYNC_MODE_TWO_WAY_RESOLVED,
};
use crate::tooling::tailscale::{parse_status_json, status_args};

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

fn has_matching_existing_sync(
    existing_syncs: &[ListedSession],
    root: &SyncRootInput,
) -> Result<bool> {
    let matching_name = existing_syncs
        .iter()
        .filter(|session| session.name == root.name)
        .collect::<Vec<_>>();

    if matching_name.is_empty() {
        return Ok(false);
    }

    if matching_name.len() > 1 {
        return Err(anyhow!(
            "multiple mutagen syncs already use the name `{}`; remove the duplicates before rerunning `eternalMac setup client`",
            root.name
        ));
    }

    let existing = matching_name[0];
    if existing.alpha_url.as_deref() == Some(root.local.as_str())
        && existing.beta_url.as_deref() == Some(root.remote.as_str())
    {
        return Ok(true);
    }

    Err(anyhow!(
        "existing mutagen sync `{}` does not match requested endpoints; expected {} <-> {}, found {} <-> {}",
        root.name,
        root.local,
        root.remote,
        existing.alpha_url.as_deref().unwrap_or("unknown"),
        existing.beta_url.as_deref().unwrap_or("unknown")
    ))
}

fn current_executable_path() -> Result<String> {
    Ok(std::env::current_exe()
        .context("resolving current executable path for client launch agent")?
        .display()
        .to_string())
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

    let tailscale_args = status_args();
    let tailscale_status = run_checked(runner, "tailscale", &tailscale_args)?;
    let parsed_status = parse_status_json(&tailscale_status.stdout)?;
    let tailscale_ok = parsed_status.backend_state == "Running";

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

    let config = Config {
        role: Role::Client,
        server: None,
        client: Some(ClientConfig {
            paired_server: input.paired_server.clone(),
            pinned: vec![],
            sync_pairs: sync_pairs.clone(),
        }),
        session: SessionConfig { auto_attach: true },
    };
    let mut sync_states = sync_pairs
        .iter()
        .map(|pair| SyncPairState {
            name: pair.name.clone(),
            local: pair.local.clone(),
            remote: pair.remote.clone(),
            mode: pair.mode.clone(),
            status: "pending".into(),
        })
        .collect::<Vec<_>>();
    persist_config_and_state(
        paths,
        store,
        &config,
        &State {
            role: Role::Client,
            tailscale_ok,
            server_reachable: false,
            healthy: false,
            summary: "client setup in progress; finalizing sync prerequisites".into(),
            tailscale_dns: None,
            daemon_healthy: false,
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: sync_states.clone(),
        },
    )?;

    let existing_syncs_output = run_checked(runner, "mutagen", &mutagen_list_args())?;
    let existing_syncs = parse_list_output(&existing_syncs_output.stdout);

    let mut created_sync_count = 0usize;
    for (index, root) in input.sync_roots.iter().enumerate() {
        if !has_matching_existing_sync(&existing_syncs, root)? {
            let args = build_create_args(&root.name, &root.local, &root.remote);
            run_checked(runner, "mutagen", &args)?;
        }
        sync_states[index].status = "created".into();
        created_sync_count += 1;
        store.save_state(&State {
            role: Role::Client,
            tailscale_ok,
            server_reachable: created_sync_count > 0,
            healthy: false,
            summary: "client setup in progress; finalizing sync prerequisites".into(),
            tailscale_dns: None,
            daemon_healthy: false,
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: sync_states.clone(),
        })?;
    }

    let sync_names = sync_pairs
        .iter()
        .map(|pair| pair.name.clone())
        .collect::<Vec<_>>();
    let executable_path = current_executable_path()?;

    write_plist(
        &paths.client_plist,
        &Definition {
            label: "com.eternalmac.client".into(),
            program_arguments: vec![executable_path, "daemon".into(), "client".into()],
            run_at_load: true,
            keep_alive: true,
        },
    )?;

    run_unload_checked(runner, &paths.server_plist)?;

    let launchctl_args = vec![
        "load".into(),
        "-w".into(),
        paths.client_plist.display().to_string(),
    ];
    run_checked(runner, "launchctl", &launchctl_args)?;

    let server_reachable = !sync_pairs.is_empty();
    let healthy = tailscale_ok && server_reachable;
    store.save_state(&State {
        role: Role::Client,
        tailscale_ok,
        server_reachable,
        healthy,
        summary: "client setup complete; runtime health pending".into(),
        tailscale_dns: None,
        daemon_healthy: false,
        daemon_heartbeat_unix: 0,
        default_session_present: false,
        known_sessions: vec![],
        syncs: sync_states,
    })?;

    Ok(ClientSetupSummary {
        paired_server: input.paired_server,
        sync_names,
    })
}
