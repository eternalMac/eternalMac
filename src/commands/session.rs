use anyhow::{anyhow, Context, Result};

use crate::app::context::AppContext;
use crate::config::store::Store;
use crate::process::runner::{Output, Runner};
use crate::session::service;
use crate::tooling::tmux::{list_sessions_args, new_session_args, parse_sessions};

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

pub fn list_with<R: Runner>(runner: &R) -> Result<Vec<String>> {
    let output = run_checked(runner, "tmux", &list_sessions_args())?;
    Ok(parse_sessions(&output.stdout))
}

pub fn create_with<R: Runner>(runner: &R, name: &str) -> Result<()> {
    run_checked(runner, "tmux", &new_session_args(name))?;
    Ok(())
}

pub fn pin_session_with(store: &Store, name: &str) -> Result<Vec<String>> {
    let mut config = store
        .load_config()
        .context("loading config for session pin")?;
    if let Some(server) = config.server.as_mut() {
        server.boot_sessions = service::pin(server.boot_sessions.clone(), name);
    }
    if let Some(client) = config.client.as_mut() {
        client.pinned = service::pin(client.pinned.clone(), name);
    }

    let pinned = config
        .client
        .as_ref()
        .map(|client| client.pinned.clone())
        .or_else(|| {
            config
                .server
                .as_ref()
                .map(|server| server.boot_sessions.clone())
        })
        .unwrap_or_default();
    store
        .save_config(&config)
        .context("saving config after session pin")?;

    Ok(pinned)
}

pub fn unpin_session_with(store: &Store, name: &str) -> Result<Vec<String>> {
    let mut config = store
        .load_config()
        .context("loading config for session unpin")?;
    if let Some(server) = config.server.as_mut() {
        server.boot_sessions = service::unpin(server.boot_sessions.clone(), name);
    }
    if let Some(client) = config.client.as_mut() {
        client.pinned = service::unpin(client.pinned.clone(), name);
    }

    let pinned = config
        .client
        .as_ref()
        .map(|client| client.pinned.clone())
        .or_else(|| {
            config
                .server
                .as_ref()
                .map(|server| server.boot_sessions.clone())
        })
        .unwrap_or_default();
    store
        .save_config(&config)
        .context("saving config after session unpin")?;

    Ok(pinned)
}

pub fn list() -> Result<()> {
    let context = AppContext::from_env()?;
    for session_name in list_with(&context.runner)? {
        println!("{session_name}");
    }
    Ok(())
}

pub fn create(name: &str) -> Result<()> {
    let context = AppContext::from_env()?;
    create_with(&context.runner, name)
}

pub fn pin_session(name: &str) -> Result<()> {
    let context = AppContext::from_env()?;
    pin_session_with(&context.store, name)?;
    Ok(())
}

pub fn unpin_session(name: &str) -> Result<()> {
    let context = AppContext::from_env()?;
    unpin_session_with(&context.store, name)?;
    Ok(())
}
