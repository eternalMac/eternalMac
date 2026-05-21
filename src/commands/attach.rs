use anyhow::{anyhow, Context, Result};

use crate::app::context::AppContext;
use crate::config::store::Store;
use crate::process::runner::{Output, Runner};
use crate::tooling::et::{build_attach_args_with_options, build_new_session_args_with_options};

const DEFAULT_SESSION: &str = "default";

pub fn resolve_session_name(session: Option<&str>) -> String {
    let selected = session.map(str::trim).unwrap_or(DEFAULT_SESSION);
    if selected.is_empty() {
        return DEFAULT_SESSION.into();
    }

    selected.into()
}

fn run_checked<R: Runner>(
    runner: &R,
    program: &str,
    args: &[String],
    interactive: bool,
) -> Result<Output> {
    let output = if interactive {
        runner.run_interactive(program, args)?
    } else {
        runner.run(program, args)?
    };
    if output.success {
        return Ok(output);
    }

    let mut message = format!("command failed: {program} {}", args.join(" "));
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }

    Err(anyhow!(message))
}

pub fn run_with<R: Runner>(
    store: &Store,
    runner: &R,
    session: Option<&str>,
    new_session: Option<&str>,
) -> Result<()> {
    if session.is_some() && new_session.is_some() {
        return Err(anyhow!(
            "attach accepts either an existing session or --new, not both"
        ));
    }

    let config = store
        .load_config()
        .context("loading client config for attach")?;
    let client = config.client.as_ref().context(
        "attach requires client config; run `eternalMac setup client` on this machine first",
    )?;

    let session = match new_session {
        Some(new_session) => {
            let session = resolve_session_name(Some(new_session));
            let args = build_new_session_args_with_options(
                &client.paired_server,
                client.server_ssh_user.as_deref(),
                client.server_etterminal_path.as_deref(),
                &session,
            );
            run_checked(runner, "et", &args, false)?;
            session
        }
        None => resolve_session_name(session),
    };

    let args = build_attach_args_with_options(
        &client.paired_server,
        client.server_ssh_user.as_deref(),
        client.server_etterminal_path.as_deref(),
        &session,
    );
    run_checked(runner, "et", &args, true)?;
    Ok(())
}

pub fn run(session: Option<&str>, new_session: Option<&str>) -> Result<()> {
    let context = AppContext::from_env()?;
    run_with(&context.store, &context.runner, session, new_session)
}
