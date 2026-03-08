// CLI module for NewClaw

use std::io::{self, Write};
use crate::core::AgentEngine;
use crate::llm::GLMProvider;

pub async fn run_cli() -> anyhow::Result<()> {
    println!("🦀 NewClaw v{} - Interactive Mode", env!("CARGO_PKG_VERSION"));
    println!("Type 'exit' or 'quit' to exit\n");
    
    // Check for GLM API key
    let api_key = std::env::var("GLM_API_KEY").ok();
    
    let mut agent = if let Some(key) = api_key {
        println!("✅ GLM API key found");
        let provider = Box::new(GLMProvider::new(key));
        AgentEngine::new(
            "NewClaw".to_string(),
            "glm-4".to_string(),
        )?
        .with_llm(provider)
    } else {
        println!("⚠️  No GLM_API_KEY found, running in mock mode");
        println!("   Set GLM_API_KEY environment variable to enable LLM");
        AgentEngine::new(
            "NewClaw".to_string(),
            "glm-4".to_string(),
        )?
    };
    
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
