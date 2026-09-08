//! Arg parsing, mode dispatch.

use anyhow::bail;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "start-page")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// HTTP daemon + browser UI
    Serve,
    /// Terminal UI
    Tui,
    /// Sync providers
    Sync,
    /// Config subcommands
    Config,
    /// Excalith JSON importer
    Import,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve => bail!("not implemented"),
        Command::Tui => bail!("not implemented"),
        Command::Sync => bail!("not implemented"),
        Command::Config => bail!("not implemented"),
        Command::Import => bail!("not implemented"),
    }
}
