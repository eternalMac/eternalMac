use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{attach, daemon, doctor, session, setup::SetupCommand, status, sync};

#[derive(Debug, Parser)]
#[command(name = "eternalMac")]
#[command(about = "Turn a Mac Mini into a personal devserver")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Configure the server or client")]
    Setup {
        #[command(subcommand)]
        target: SetupCommand,
    },
    #[command(about = "Attach to a session")]
    Attach { session: Option<String> },
    #[command(about = "Show the current server status")]
    Status,
    #[command(about = "Run health checks on the local machine")]
    Doctor,
    #[command(about = "Manage sessions")]
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    #[command(about = "Manage sync pairs")]
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
    #[command(about = "Run daemon commands")]
    #[command(hide = true)]
    Daemon {
        #[command(subcommand)]
        target: DaemonAction,
    },
}

#[derive(Debug, Subcommand)]
enum SessionAction {
    #[command(about = "List sessions")]
    List,
    #[command(about = "Create a new session")]
    New { name: String },
    #[command(about = "Pin a session")]
    Pin { name: String },
    #[command(about = "Unpin a session")]
    Unpin { name: String },
}

#[derive(Debug, Subcommand)]
enum SyncAction {
    #[command(about = "Add a sync pair")]
    Add {
        name: String,
        #[arg(long)]
        local: String,
        #[arg(long)]
        remote: String,
    },
    #[command(about = "List sync pairs")]
    List,
    #[command(about = "Show sync status")]
    Status,
}

#[derive(Debug, Subcommand)]
enum DaemonAction {
    #[command(about = "Start the daemon server")]
    Server,
    #[command(about = "Start the daemon client")]
    Client,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Setup {
            target: SetupCommand::Server,
        }) => println!("server plan ready for mac-mini"),
        Some(Command::Setup {
            target: SetupCommand::Client { server },
        }) => println!("client plan ready for {server}"),
        Some(Command::Attach { session }) => attach::run(session.as_deref()),
        Some(Command::Status) => status::run(),
        Some(Command::Doctor) => doctor::run(),
        Some(Command::Session {
            action: SessionAction::List,
        }) => session::list(),
        Some(Command::Session {
            action: SessionAction::New { name },
        }) => session::create(&name),
        Some(Command::Session {
            action: SessionAction::Pin { name },
        }) => session::pin_session(&name),
        Some(Command::Session {
            action: SessionAction::Unpin { name },
        }) => session::unpin_session(&name),
        Some(Command::Sync {
            action:
                SyncAction::Add {
                    name,
                    local,
                    remote,
                },
        }) => sync::add(&name, &local, &remote),
        Some(Command::Sync {
            action: SyncAction::List,
        }) => sync::list(),
        Some(Command::Sync {
            action: SyncAction::Status,
        }) => sync::status(),
        Some(Command::Daemon {
            target: DaemonAction::Server,
        }) => daemon::run_server(),
        Some(Command::Daemon {
            target: DaemonAction::Client,
        }) => daemon::run_client(),
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
        }
    }
    Ok(())
}
