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
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Excalith JSON importer
    Import,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the resolved config file path
    Path,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve => bail!("not implemented"),
        Command::Tui => bail!("not implemented"),
        Command::Sync => bail!("not implemented"),
        Command::Config { command } => match command {
            ConfigCommand::Path => {
                println!("{}", sp_config::config_path()?.display());
                Ok(())
            }
        },
        Command::Import => bail!("not implemented"),
    }
}
