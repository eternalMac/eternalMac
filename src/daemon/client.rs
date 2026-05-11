use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};

use crate::app::paths::Paths;
use crate::config::store::Store;
use crate::model::config::Role;
use crate::model::state::{State, SyncPairState};
use crate::process::runner::{Output, Runner};
use crate::tooling::mutagen::list_args;
use crate::tooling::tailscale::{parse_status_json, status_args};

#[derive(Debug, Clone)]
pub struct ClientInput {
    pub pinned_sessions: Vec<String>,
    pub tailscale_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientState {
    pub auto_attach: Vec<String>,
    pub healthy: bool,
}

pub fn reconcile_client(input: ClientInput) -> ClientState {
    if !input.tailscale_ready {
        return ClientState {
            auto_attach: vec![],
            healthy: false,
        };
    }
    ClientState {
        auto_attach: input.pinned_sessions,
        healthy: true,
    }
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

fn current_unix_seconds() -> Result<i64> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    i64::try_from(seconds).context("unix timestamp overflow converting u64 to i64")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MutagenSyncStatus {
    name: String,
    status: String,
}

fn parse_sync_statuses(mutagen_output: &str) -> Vec<MutagenSyncStatus> {
    fn push_current(
        parsed: &mut Vec<MutagenSyncStatus>,
        current_name: &mut Option<String>,
        current_status: &mut Option<String>,
    ) {
        if let (Some(name), Some(status)) = (current_name.take(), current_status.take()) {
            parsed.push(MutagenSyncStatus { name, status });
        }
    }

    let mut parsed = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_status: Option<String> = None;

    for raw_line in mutagen_output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            push_current(&mut parsed, &mut current_name, &mut current_status);
            continue;
        }

        if let Some(name) = line.strip_prefix("Name:") {
            push_current(&mut parsed, &mut current_name, &mut current_status);
            current_name = Some(name.trim().to_string());
            continue;
        }

        if let Some(status) = line.strip_prefix("Status:") {
            current_status = Some(status.trim().to_string());
        }
    }

    push_current(&mut parsed, &mut current_name, &mut current_status);
    parsed
}

fn status_is_active(status: &str) -> bool {
    let normalized = status.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    ![
        "paused",
        "halted",
        "problem",
        "error",
        "stopped",
        "disconnected",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn sync_status(parsed_syncs: &[MutagenSyncStatus], sync_name: &str) -> String {
    let Some(entry) = parsed_syncs.iter().find(|sync| sync.name == sync_name) else {
        return "missing".into();
    };

    if status_is_active(&entry.status) {
        return "active".into();
    }

    "degraded".into()
}

pub fn save_failure_state(store: &Store, error: &anyhow::Error) -> Result<()> {
    let existing_state = store.load_state().ok();

    store.save_state(&State {
        role: Role::Client,
        tailscale_ok: false,
        server_reachable: false,
        healthy: false,
        summary: format!("client daemon degraded: {error}"),
        tailscale_dns: existing_state
            .as_ref()
            .and_then(|state| state.tailscale_dns.clone()),
        daemon_healthy: false,
        daemon_heartbeat_unix: current_unix_seconds()?,
        default_session_present: false,
        known_sessions: vec![],
        syncs: existing_state.map(|state| state.syncs).unwrap_or_default(),
    })?;

    Ok(())
}

pub fn run_once<R: Runner>(_paths: &Paths, store: &Store, runner: &R) -> Result<()> {
    let config = store.load_config()?;
    let client = config
        .client
        .as_ref()
        .context("client daemon requires client config")?;

    let tailscale_output = run_checked(runner, "tailscale", &status_args())?;
    let tailscale_status = parse_status_json(&tailscale_output.stdout)?;
    let tailscale_ok = tailscale_status.backend_state == "Running";

    let mutagen_output = run_checked(runner, "mutagen", &list_args())?;
    let parsed_syncs = parse_sync_statuses(&mutagen_output.stdout);
    let syncs = client
        .sync_pairs
        .iter()
        .map(|sync_pair| SyncPairState {
            name: sync_pair.name.clone(),
            local: sync_pair.local.clone(),
            remote: sync_pair.remote.clone(),
            mode: sync_pair.mode.clone(),
            status: sync_status(&parsed_syncs, &sync_pair.name),
        })
        .collect::<Vec<_>>();
    let all_syncs_active = syncs.iter().all(|sync| sync.status == "active");
    let healthy = tailscale_ok && all_syncs_active;

    store.save_state(&State {
        role: config.role,
        tailscale_ok,
        server_reachable: tailscale_ok,
        healthy,
        summary: if healthy {
            "client daemon healthy".into()
        } else {
            "client daemon degraded".into()
        },
        tailscale_dns: tailscale_status.dns_name,
        daemon_healthy: true,
        daemon_heartbeat_unix: current_unix_seconds()?,
        default_session_present: false,
        known_sessions: vec![],
        syncs,
    })?;

    Ok(())
}
