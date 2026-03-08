// NewClaw CLI

use clap::{Parser, Subcommand};
use std::net::SocketAddr;

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
    Gateway {
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
    },
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
        Commands::Gateway { port, host } => {
            println!("Starting Web Gateway on {}:{}...", host, port);
            
            let app = newclaw::gateway::create_router();
            
            let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
            let listener = tokio::net::TcpListener::bind(addr).await?;
            
            println!("✅ Gateway listening on http://{}", addr);
            println!("   Health check: http://{}/health", addr);
            println!("   Chat endpoint: http://{}/chat", addr);
            
            axum::serve(listener, app).await?;
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
