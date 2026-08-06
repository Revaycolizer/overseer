//! Live end-to-end demo of the governed agent loop on the local model.
//!
//! Runs a real goal through: local Ollama function-calling → kernel policy
//! (autonomous auto-approve) → tool execution → observation → summary.
//!
//!   cargo run --example agent_live -- "List the files in /tmp and count them"
//!
//! Uses an in-memory DB and (for an unattended demo) `full_auto` supervision so
//! it never blocks on approval. In real use, `autonomous` auto-approves only
//! low-risk actions and pauses for a human on anything riskier.

use overseer::db::Database;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("warn"))
        .init();

    let db = Database::new_in_memory().await.expect("db");
    // Unattended demo: auto-approve so the run completes without a human. Swap to
    // "autonomous" to see low-risk auto-run and riskier steps pause for approval.
    let mode = std::env::var("OVERSEER_DEMO_MODE").unwrap_or_else(|_| "full_auto".into());
    overseer::backend::set_setting(&db, "computer_supervision_mode", &mode)
        .await
        .expect("set mode");

    let conv = overseer::backend::add_conversation(&db, "live-demo").await.expect("conv");
    let goal = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "List the files in /tmp using the shell, then tell me how many there are.".to_string());

    println!("── Overseer live agent demo ──");
    println!("mode: {mode}");
    println!("goal: {goal}\n");

    let out = overseer::engine::agent_loop::run_agent(&db, conv, &goal, None).await;

    // run_agent returns a StructuredResponse JSON; print the assistant message.
    match serde_json::from_str::<serde_json::Value>(&out) {
        Ok(v) => {
            let status = v["status"].as_str().unwrap_or("?");
            let msg = v["messages"][0]["message"].as_str().unwrap_or(&out);
            println!("── result [{status}] ──\n{msg}");
        }
        Err(_) => println!("{out}"),
    }
}
