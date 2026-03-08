// CLI module for NewClaw

use std::io::{self, Write};
use crate::core::AgentEngine;

pub async fn run_cli() -> anyhow::Result<()> {
    println!("🦀 NewClaw v{} - Interactive Mode", env!("CARGO_PKG_VERSION"));
    println!("Type 'exit' or 'quit' to exit\n");
    
    let mut agent = AgentEngine::new(
        "NewClaw".to_string(),
        "glm-4".to_string(),
    )?;
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        if input == "exit" || input == "quit" {
            println!("Goodbye!");
            break;
        }
        
        match agent.process(input).await {
            Ok(response) => {
                println!("🤖 {}", response);
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
            }
        }
    }
    
    Ok(())
}
