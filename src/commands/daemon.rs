use std::thread;
use std::time::Duration;

use anyhow::Result;

use crate::app::context::AppContext;

const DAEMON_INTERVAL: Duration = Duration::from_secs(30);

fn run_server_iteration(context: &AppContext) {
    if let Err(error) =
        crate::daemon::server::run_once(&context.paths, &context.store, &context.runner)
    {
        eprintln!("server daemon iteration failed: {error:#}");
        if let Err(save_error) = crate::daemon::server::save_failure_state(&context.store, &error) {
            eprintln!("server daemon state snapshot failed: {save_error:#}");
        }
    }
}

fn run_client_iteration(context: &AppContext) {
    if let Err(error) =
        crate::daemon::client::run_once(&context.paths, &context.store, &context.runner)
    {
        eprintln!("client daemon iteration failed: {error:#}");
        if let Err(save_error) = crate::daemon::client::save_failure_state(&context.store, &error) {
            eprintln!("client daemon state snapshot failed: {save_error:#}");
        }
    }
}

pub fn run_server() -> Result<()> {
    let context = AppContext::from_env()?;
    loop {
        run_server_iteration(&context);
        thread::sleep(DAEMON_INTERVAL);
    }
}

pub fn run_client() -> Result<()> {
    let context = AppContext::from_env()?;
    loop {
        run_client_iteration(&context);
        thread::sleep(DAEMON_INTERVAL);
    }
}
