use anyhow::{anyhow, Context, Result};

use crate::app::context::AppContext;
use crate::config::store::Store;
use crate::model::config::Config;
use crate::process::runner::{command_failed, Output, Runner};
use crate::session::service;
use crate::tooling::et::{
    build_list_sessions_args_with_options as build_remote_list_sessions_args,
    build_new_session_args_with_options as build_remote_new_session_args,
    SESSION_LIST_BEGIN_MARKER, SESSION_LIST_END_MARKER,
};
use crate::tooling::tmux::{list_sessions_args, new_session_args, parse_sessions};

fn run_checked<R: Runner>(runner: &R, program: &str, args: &[String]) -> Result<Output> {
    let output = runner.run(program, args)?;
    if output.success {
        return Ok(output);
    }

    Err(command_failed(program, args, &output))
}

enum SessionExecutionTarget<'a> {
    Local,
    Remote {
        server: &'a str,
        server_ssh_user: Option<&'a str>,
        server_etterminal_path: Option<&'a str>,
    },
}

fn execution_target(config: &Config) -> Result<SessionExecutionTarget<'_>> {
    if let Some(client) = config.client.as_ref() {
        return Ok(SessionExecutionTarget::Remote {
            server: &client.paired_server,
            server_ssh_user: client.server_ssh_user.as_deref(),
            server_etterminal_path: client.server_etterminal_path.as_deref(),
        });
    }

    if config.server.is_some() {
        return Ok(SessionExecutionTarget::Local);
    }

    Err(anyhow!(
        "session commands require setup: run `eternalMac setup server` or `eternalMac setup client`"
    ))
}

fn parse_marked_remote_sessions(stdout: &str) -> Vec<String> {
    let mut collecting = false;
    let mut sessions = Vec::new();

    for line in stdout.lines().map(str::trim) {
        if line == SESSION_LIST_BEGIN_MARKER {
            collecting = true;
            sessions.clear();
            continue;
        }
        if line == SESSION_LIST_END_MARKER {
            break;
        }
        if collecting && !line.is_empty() {
            sessions.push(line.to_string());
        }
    }

    sessions
}

pub fn list_with<R: Runner>(store: &Store, runner: &R) -> Result<Vec<String>> {
    let config = store
        .load_config()
        .context("loading config for session list")?;

    match execution_target(&config)? {
        SessionExecutionTarget::Local => {
            let output = run_checked(runner, "tmux", &list_sessions_args())?;
            Ok(parse_sessions(&output.stdout))
        }
        SessionExecutionTarget::Remote {
            server,
            server_ssh_user,
            server_etterminal_path,
        } => {
            let output = run_checked(
                runner,
                "et",
                &build_remote_list_sessions_args(server, server_ssh_user, server_etterminal_path),
            )?;
            Ok(parse_marked_remote_sessions(&output.stdout))
        }
    }
}

pub fn create_with<R: Runner>(store: &Store, runner: &R, name: &str) -> Result<()> {
    let config = store
        .load_config()
        .context("loading config for session create")?;

    match execution_target(&config)? {
        SessionExecutionTarget::Local => run_checked(runner, "tmux", &new_session_args(name))?,
        SessionExecutionTarget::Remote {
            server,
            server_ssh_user,
            server_etterminal_path,
        } => run_checked(
            runner,
            "et",
            &build_remote_new_session_args(server, server_ssh_user, server_etterminal_path, name),
        )?,
    };

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
    for session_name in list_with(&context.store, &context.runner)? {
        println!("{session_name}");
    }
    Ok(())
}

pub fn create(name: &str) -> Result<()> {
    let context = AppContext::from_env()?;
    create_with(&context.store, &context.runner, name)
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
