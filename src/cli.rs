use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{doctor, setup::SetupCommand, status};

#[derive(Debug, Parser)]
#[command(name = "eternalMac")]
#[command(about = "Turn a Mac Mini into a personal devserver")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Setup {
        #[command(subcommand)]
        target: SetupCommand,
    },
    Status,
    Doctor,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Setup {
            target: SetupCommand::Server,
        }) => {
            println!("server plan ready for mac-mini");
        }
        Some(Command::Setup {
            target: SetupCommand::Client { server },
        }) => {
            println!("client plan ready for {server}");
        }
        Some(Command::Status) => status::run(),
        Some(Command::Doctor) => doctor::run(),
        None => {
            Cli::parse_from(["eternalMac", "--help"]);
        }
    }
    Ok(())
}
