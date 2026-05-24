use anyhow::{anyhow, Context, Result};

use crate::app::context::AppContext;
use crate::config::store::Store;
use crate::model::config::{ClientConfig, SyncPairConfig};
use crate::process::runner::{Output, Runner};
use crate::sync::service::build_pair;
use crate::tooling::mutagen::{build_create_args, list_args};
use crate::tooling::ssh::build_sync_destination;

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

fn has_remote_host(endpoint: &str) -> bool {
    if endpoint.contains("://") {
        return true;
    }

    if endpoint.starts_with('/') || endpoint.starts_with("~/") || endpoint.starts_with("./") {
        return false;
    }

    let Some(colon_index) = endpoint.find(':') else {
        return false;
    };

    endpoint
        .find('/')
        .is_none_or(|slash_index| colon_index < slash_index)
}

fn resolve_remote_endpoint(client: &ClientConfig, remote: &str) -> String {
    if has_remote_host(remote) {
        return remote.into();
    }

    if let Some(server_ssh_user) = client
        .server_ssh_user
        .as_deref()
        .filter(|server_ssh_user| !server_ssh_user.trim().is_empty())
    {
        return build_sync_destination(server_ssh_user, &client.paired_server, remote);
    }

    format!("{}:{remote}", client.paired_server)
}

pub fn add_with<R: Runner>(
    store: &Store,
    runner: &R,
    name: &str,
    local: &str,
    remote: &str,
) -> Result<SyncPairConfig> {
    let mut config = store
        .load_config()
        .context("loading client config for sync add")?;
    let client = config.client.as_mut().context(
        "sync add requires client config; run `eternalMac setup client` on this machine first",
    )?;
    let remote = resolve_remote_endpoint(client, remote);
    let pair = build_pair(name, local, &remote, None);

    let sync_pair = SyncPairConfig {
        name: pair.name,
        local: pair.local,
        remote: pair.remote,
        mode: pair.mode,
    };
    if let Some(existing) = client
        .sync_pairs
        .iter_mut()
        .find(|existing| existing.name == sync_pair.name)
    {
        if existing.local != sync_pair.local || existing.remote != sync_pair.remote {
            return Err(anyhow!(
                "sync add found existing pair `{}` with different endpoints; use a different name or remove the existing pair first",
                sync_pair.name
            ));
        }
        *existing = sync_pair.clone();
    } else {
        client.sync_pairs.push(sync_pair.clone());
    }

    store
        .save_config(&config)
        .context("saving client config after sync add")?;

    run_checked(
        runner,
        "mutagen",
        &build_create_args(&sync_pair.name, &sync_pair.local, &sync_pair.remote),
    )?;

    Ok(sync_pair)
}

pub fn list_with(store: &Store) -> Result<Vec<SyncPairConfig>> {
    let config = store
        .load_config()
        .context("loading client config for sync list")?;
    let client = config.client.as_ref().context(
        "sync list requires client config; run `eternalMac setup client` on this machine first",
    )?;
    Ok(client.sync_pairs.clone())
}

pub fn status_with<R: Runner>(runner: &R) -> Result<String> {
    let output = run_checked(runner, "mutagen", &list_args())?;
    Ok(output.stdout)
}

pub fn add(name: &str, local: &str, remote: &str) -> Result<()> {
    let context = AppContext::from_env()?;
    add_with(&context.store, &context.runner, name, local, remote)?;
    Ok(())
}

pub fn list() -> Result<()> {
    let context = AppContext::from_env()?;
    for pair in list_with(&context.store)? {
        println!("{} {} {}", pair.name, pair.local, pair.remote);
    }
    Ok(())
}

pub fn status() -> Result<()> {
    let context = AppContext::from_env()?;
    print!("{}", status_with(&context.runner)?);
    Ok(())
}
