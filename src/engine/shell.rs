use rig::providers::ollama;
use rig::client::Nothing;
use std::io::{self, Write};

use crate::engine::agents::master_planner::MasterPlanner;
use crate::engine::memory::{setup_memory_table, retrieve_memories, store_memory};

pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let sqlite_db = crate::db::Database::new().await?;
    let system = crate::engine::system::AgentSystem::new(sqlite_db.clone());
    
    // Create a special CLI conversation if it doesn't exist
    let conversation_id = match crate::backend::get_conversations(&sqlite_db).await {
        Ok(convs) => convs.iter().find(|c| c.title == "CLI Conversation").map(|c| c.id),
        _ => None,
    };
    
    let conversation_id = if let Some(id) = conversation_id {
        id
    } else {
        crate::backend::add_conversation(&sqlite_db, "CLI Conversation").await.unwrap_or(0)
    };

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let first_arg = args[1].to_lowercase();
        if first_arg == "info" || first_arg == "--info" {
            crate::engine::utils::discovery::print_system_info(&sqlite_db).await;
            return Ok(());
        }
        
        let query = args[1..].join(" ");
        println!("🚀 CLI Query: {}", query);
        
        let response = system.orchestrate(&sqlite_db, conversation_id, &query).await;
        println!("\n✅ FINAL RESULT:\n{}", response);
        
        return Ok(());
    }

    println!("Welcome to the Autonomous Rig Agent (Planner-Executor Mode)!");
    println!("Features: Grounded Execution | Anti-Hallucination | RAG Memory | Code/HTTP Tools");
    println!("Type 'exit' or 'quit' to leave.\n");
    
    loop {
        print!("> "); io::stdout().flush()?;
        let mut input = String::new(); io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() { continue; }
        if trimmed == "exit" || trimmed == "quit" { break; }

        let _ = crate::backend::add_message(&sqlite_db, conversation_id, "user", trimmed).await;
        let response = system.orchestrate(&sqlite_db, conversation_id, trimmed).await;
        let _ = crate::backend::add_message(&sqlite_db, conversation_id, "assistant", &response).await;
        
        println!("\n🤖 Assistant: {}", response);
    }
    Ok(())
}
