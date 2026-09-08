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
    Serve {
        /// Bind to 0.0.0.0 instead of localhost, exposing the dashboard on the network.
        #[arg(long)]
        expose: bool,
        /// TCP port to listen on.
        #[arg(long, default_value_t = 7878)]
        port: u16,
    },
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
    Import {
        /// Path to Excalith settings.json file
        path: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the resolved config file path
    Path,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve { expose, port } => {
            let mut policy = sp_server::BindPolicy::default().with_port(port);
            if expose {
                policy = policy.expose(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
            }
            sp_server::Server::run(policy)?;
            Ok(())
        }
        Command::Tui => Ok(sp_tui_host::run()?),
        Command::Sync => bail!("not implemented"),
        Command::Config { command } => match command {
            ConfigCommand::Path => {
                println!("{}", sp_config::config_path()?.display());
                Ok(())
            }
        },
        Command::Import { path } => {
            let contents = std::fs::read_to_string(&path)?;
            let config = sp_config::Config::from_excalith_json(&contents)?;
            let target = sp_config::config_path()?;
            sp_config::save_to(&config, &target)?;
            println!("Imported {} -> {}", path.display(), target.display());
            Ok(())
        }
    }
}
