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

pub async fn dispatch_domain(
    db: &crate::db::Database,
    domain: &str,
    step: &str,
    query: &str,
    memories: &str,
    plan_history: &str,
    memory_context: &str,
    macro_task_description: &str,
    web_supervisor: &WebSupervisor,
    app_supervisor: &AppSupervisor,
    office_supervisor: &OfficeSupervisor,
    text_writer_supervisor: &TextWriterSupervisor,
    neontune_supervisor: &NeontuneSupervisor,
    overseer: &OverseerAgent,
    chat_agent: &ChatAgent,
    stealth_supervisor: &StealthSupervisor,
    perception_tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    browser_pid: &mut Option<i32>,
) -> Result<String, String> {
    match domain {
        "web" => {
            println!("🌐 DISPATCH: WebSupervisor -> run_loop");
            web_supervisor.run_loop(db, step, Vec::new(), memories, plan_history, perception_tool, ollama_client, overseer, browser_pid).await
        },
        "office" => {
            println!("🏢 DISPATCH: OfficeSupervisor -> run_loop");
            office_supervisor.run_loop(db, overseer, step, perception_tool, ollama_client, plan_history, memory_context).await
        },
        "text_writer" => {
            println!("📝 DISPATCH: TextWriterSupervisor -> run_loop");
            text_writer_supervisor.run_loop(db, overseer, step, perception_tool, ollama_client, plan_history, memory_context).await
        },
        "neontune" => {
            println!("🎵 DISPATCH: NeontuneSupervisor -> run_loop");
            neontune_supervisor.run_loop(db, overseer, query, perception_tool, plan_history, macro_task_description, step).await
        },
        "chat" => {
            println!("💬 DISPATCH: ChatAgent -> answer");
            Ok(chat_agent.answer(step, memory_context).await)
        },
        "stealth" => {
            println!("👤 DISPATCH: StealthSupervisor -> run_loop");
            stealth_supervisor.run_loop(db, step, Vec::new(), memory_context, perception_tool).await
        },
        _ => {
            println!("🖥️ DISPATCH: AppSupervisor -> run_loop");
            app_supervisor.run_loop(db, step, memories, plan_history, perception_tool, overseer).await
        }
    }
}
