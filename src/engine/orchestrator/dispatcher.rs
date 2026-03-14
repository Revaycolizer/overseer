use rig::providers::ollama;
use crate::engine::agents::{
    web::supervisor::WebSupervisor,
    office::supervisor::OfficeSupervisor,
    desktop::supervisor::AppSupervisor,
    neontune::supervisor::NeontuneSupervisor,
    text_writer::supervisor::TextWriterSupervisor,
    overseer::OverseerAgent,
    chat::ChatAgent,
    stealth::StealthSupervisor,
};
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

pub async fn dispatch_domain(
    db: &crate::db::Database,
    agent_registry: &[AgentInfo],
    domain: &str,
    step: &str,
    query: &str,
    memories: &str,
    plan_history: &str,
    memory_context: &str,
    macro_task_description: &str,
    perception_tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    browser_pid: &mut Option<i32>,
) -> Result<String, String> {
    // Helper to find agent info or use default
    let get_agent = |domain_name: &str| {
        agent_registry.iter().find(|a| a.domain == domain_name)
            .cloned()
            .unwrap_or_else(|| AgentInfo {
                id: 0,
                name: domain_name.to_string(),
                description: String::new(),
                tools: Vec::new(),
                instructions: String::new(),
                model: "qwen2.5:7b-instruct".to_string(),
                domain: domain_name.to_string(),
                version: "1.0.0".to_string(),
                source_files: Vec::new(),
            })
    };

    // Instantiate overseer (needed by most supervisors)
    let overseer_info = get_agent("overseer");
    let overseer = OverseerAgent::new(ollama_client, &overseer_info.instructions);

    match domain {
        "web" => {
            println!("🌐 DISPATCH: WebSupervisor -> run_loop");
            let info = get_agent("web_supervisor");
            let supervisor = WebSupervisor::new(ollama_client, &info.instructions);
            supervisor.run_loop(db, step, Vec::new(), memories, plan_history, perception_tool, ollama_client, &overseer, browser_pid).await
        },
        "office" => {
            println!("🏢 DISPATCH: OfficeSupervisor -> run_loop");
            let info = get_agent("office_supervisor");
            let supervisor = OfficeSupervisor::new(ollama_client, &info.instructions);
            supervisor.run_loop(db, &overseer, step, perception_tool, ollama_client, plan_history, memory_context).await
        },
        "text_writer" => {
            println!("📝 DISPATCH: TextWriterSupervisor -> run_loop");
            let info = get_agent("text_writer");
            let supervisor = TextWriterSupervisor::new(ollama_client, &info.instructions);
            supervisor.run_loop(db, &overseer, step, perception_tool, ollama_client, plan_history, memory_context).await
        },
        "neontune" => {
            println!("🎵 DISPATCH: NeontuneSupervisor -> run_loop");
            let info = get_agent("neontune_supervisor");
            let supervisor = NeontuneSupervisor::new(ollama_client, &info.instructions);
            supervisor.run_loop(db, &overseer, query, perception_tool, plan_history, macro_task_description, step).await
        },
        "chat" => {
            println!("💬 DISPATCH: ChatAgent -> answer");
            let info = get_agent("chat");
            let agent = ChatAgent::new(ollama_client, &info.instructions);
            Ok(agent.answer(step, memory_context).await)
        },
        "stealth" => {
            println!("👤 DISPATCH: StealthSupervisor -> run_loop");
            let info = get_agent("stealth");
            let supervisor = StealthSupervisor::new(ollama_client, &info.instructions);
            supervisor.run_loop(db, step, Vec::new(), memory_context, perception_tool).await
        },
        _ => {
            println!("🖥️ DISPATCH: AppSupervisor -> run_loop");
            let info = get_agent("desktop_supervisor");
            let supervisor = AppSupervisor::new(ollama_client, &info.instructions);
            supervisor.run_loop(db, step, memories, plan_history, perception_tool, &overseer).await
        }
    }
}
