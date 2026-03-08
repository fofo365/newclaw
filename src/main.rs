// NewClaw CLI

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "newclaw")]
#[command(about = "Next-gen AI Agent framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in interactive mode
    Agent,
    /// Start web gateway
    Gateway,
    /// List plugins
    Plugin {
        #[arg(short, long)]
        list: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Agent => {
            println!("Starting NewClaw Agent...");
            newclaw::cli::run_cli().await?;
        }
        Commands::Gateway => {
            println!("Starting Web Gateway...");
            println!("TODO: Implement web gateway");
        }
        Commands::Plugin { list: true } => {
            println!("Available plugins:");
            println!("TODO: Implement plugin system");
        }
        Commands::Plugin { list: false } => {
            println!("Use --list to show plugins");
        }
    }
    
    Ok(())
}
