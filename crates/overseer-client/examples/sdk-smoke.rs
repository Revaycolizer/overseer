use overseer_client::OverseerClient;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OverseerClient::new(&std::env::var("OVERSEER_BASE_URL")?)?
        .with_token(std::env::var("OVERSEER_API_TOKEN")?);
    assert!(client.health().await?.db_connected);
    let id = client.create_conversation("Rust SDK verification").await?;
    assert!(id > 9007199254740991);
    let message = client
        .send_message(id, "user", "SDK contract check", false)
        .await?;
    assert!(client
        .list_messages(id)
        .await?
        .iter()
        .any(|m| m.id == message));
    assert!(client
        .list_conversations(50)
        .await?
        .iter()
        .any(|c| c.id == id));
    let _ = client.runtime_health().await?;
    let _ = client.system_load().await?;
    let _ = client.registry_status().await?;
    let _ = client.list_agents().await?;
    let _ = client.list_tools().await?;
    let _ = client.get_supervision_state().await?;
    let workflow = client.create_workflow("SDK verification").await?;
    let _ = client.list_workflows().await?;
    let step = client.add_workflow_step(workflow).await?;
    assert!(client
        .get_workflow_steps(workflow)
        .await?
        .iter()
        .any(|s| s.id == step));
    client.delete_workflow(workflow).await?;
    client.delete_conversation(id).await?;
    println!("Rust SDK: live REST contracts passed");
    Ok(())
}
