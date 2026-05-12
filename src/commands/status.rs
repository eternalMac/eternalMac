use anyhow::{anyhow, Result};

use crate::app::context::AppContext;
use crate::model::config::Role;
use crate::status::service::{load, render_summary, StatusLoad};

pub fn run() -> Result<()> {
    let context = AppContext::from_env()?;

    match load(&context.store)? {
        StatusLoad::Ready(snapshot) => {
            println!("{}", render_summary(&snapshot));
            Ok(())
        }
        StatusLoad::ConfigMissing => Err(anyhow!(
            "status requires setup: run `eternalMac setup server` or `eternalMac setup client`"
        )),
        StatusLoad::StateMissing { config } => Err(anyhow!(
            "status requires local state: re-run `eternalMac setup {}` to restore local state",
            setup_target(&config.role)
        )),
    }
}

fn setup_target(role: &Role) -> &'static str {
    match role {
        Role::Server => "server",
        Role::Client => "client",
    }
}
