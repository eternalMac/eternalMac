use anyhow::Result;
use clap::Subcommand;

use crate::app::context::AppContext;
use crate::setup::client::{apply_client_setup, ClientSetupInput};
use crate::setup::prompts::{prompt_server_dns, prompt_sync_roots};
use crate::setup::server::apply_server_setup;

const DEFAULT_SERVER_HOST_LABEL: &str = "mac-mini";

#[derive(Debug, Clone, Subcommand)]
pub enum SetupCommand {
    Server,
    Client {
        #[arg(long)]
        server: Option<String>,
    },
}

pub fn run_server() -> Result<()> {
    let context = AppContext::from_env()?;
    let summary = apply_server_setup(
        &context.paths,
        &context.store,
        &context.runner,
        DEFAULT_SERVER_HOST_LABEL.into(),
    )?;

    println!("Server setup complete.");
    println!("Server DNS: {}", summary.dns_name);
    println!("Default session: {}", summary.default_session);
    println!(
        "Next step: run `eternalMac setup client --server {}` on your client machine.",
        summary.dns_name
    );
    Ok(())
}

pub fn run_client(server_override: Option<String>) -> Result<()> {
    let context = AppContext::from_env()?;
    let paired_server = prompt_server_dns(server_override)?;
    let sync_roots = prompt_sync_roots(&paired_server)?;
    let summary = apply_client_setup(
        &context.paths,
        &context.store,
        &context.runner,
        ClientSetupInput {
            paired_server,
            sync_roots,
        },
    )?;

    println!("Client setup complete.");
    println!("Paired server: {}", summary.paired_server);
    println!("Sync roots: {}", summary.sync_names.join(", "));
    println!("Next step: run `eternalMac attach` to start working.");
    Ok(())
}
