//! Overseer Rust SDK Quickstart
//!
//! Run with:
//!   cargo run --example quickstart
//!
//! Or standalone:
//!   cd sdks/examples/rust-quickstart
//!   cargo run

use overseer_client::OverseerClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the server
    let client = OverseerClient::new("http://localhost:8080")?
        .with_token(std::env::var("OVERSEER_API_TOKEN").unwrap_or_default());

    // 2. Check health
    let health = client.health().await?;
    println!("Server: {} (v{})", health.status, health.version);

    // 3. List conversations
    let conversations = client.list_conversations(10).await?;
    println!("\nConversations:");
    for conv in &conversations {
        println!("  #{}: {}", conv.id, conv.title);
    }

    // 4. List agents
    let agents = client.list_agents().await?;
    println!("\nAgents:");
    for agent in &agents {
        println!("  {} ({})", agent.name, agent.domain);
    }

    // 5. List tools
    let tools = client.list_tools().await?;
    println!("\nTools:");
    for tool in &tools {
        println!("  {}: {} ({})", tool.name, tool.description, tool.language);
    }

    // 6. Create a conversation and send a message
    if conversations.is_empty() {
        let conv_id = client.create_conversation("SDK Quickstart").await?;
        println!("\nCreated conversation #{}", conv_id);

        let msg_id = client.send_message(conv_id, "user", "Hello from the Rust SDK!", false).await?;
        println!("Sent message #{}", msg_id);

        let msgs = client.list_messages(conv_id).await?;
        println!("\nMessages in conversation #{}:", conv_id);
        for msg in &msgs {
            println!("  [{}] {}", msg.role, msg.content);
        }
    }

    Ok(())
}
