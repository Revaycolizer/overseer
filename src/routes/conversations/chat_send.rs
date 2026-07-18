use crate::backend;
use crate::routes::conversations::helpers::build_query_with_context;
use crate::state::use_global_state;
use dioxus::prelude::*;
use super::chat_input::ChatInputProps;

pub struct ChatState {
    pub input_text: Signal<String>,
    pub is_local_executing: Signal<bool>,
    pub pending_attachments: Signal<Vec<(String, String, Vec<u8>)>>,
    pub is_thinking: Signal<bool>,
    pub send: Callback<()>,
    pub stop: Callback<()>,
}

pub fn use_chat_state(props: &ChatInputProps) -> ChatState {
    let global_state = use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    let agent_system = global_read.agent_system.clone();
    let client = global_read.client.clone();
    drop(global_read);

    let db_inner = db.clone();
    let client_inner = client.clone();
    let agent_inner = agent_system.clone();
    let db_for_send = db.clone();
    let client_for_send = client.clone();
    let agent_for_send = agent_system.clone();
    let db_for_stop = db.clone();

    let mut input_text: Signal<String> = use_signal(|| String::new());
    let mut is_local_executing = use_signal(|| false);
    let mut pending_attachments = use_signal(|| Vec::<(String, String, Vec<u8>)>::new());
    let mut active_request_abort_handle = use_signal(|| None::<tokio::task::JoinHandle<()>>);

    let mut messages_resource = props.messages_resource.clone();
    let mut conversations_resource = props.conversations_resource.clone();
    let mut is_thinking = props.is_thinking.clone();
    let mut selected_conversation_id = props.selected_conversation_id.clone();
    let mut selected_artifacts = props.selected_artifacts.clone();
    let mut selected_work_item = props.selected_work_item.clone();
    let mut starter_prompt = props.starter_prompt.clone();
    let is_first_msg = props.is_first_msg;

    if let Some(prompt) = starter_prompt() {
        input_text.set(prompt);
        starter_prompt.set(None);
    }

    let s_input = input_text.clone();
    let s_exec = is_local_executing.clone();
    let s_attach = pending_attachments.clone();
    let s_think = is_thinking.clone();
    let s_msgs = messages_resource.clone();
    let s_convs = conversations_resource.clone();
    let s_sel = selected_conversation_id.clone();
    let s_art = selected_artifacts.clone();
    let s_wi = selected_work_item.clone();
    let s_abort = active_request_abort_handle.clone();
    let s_first = is_first_msg;
    let s_db = db.clone();
    let s_client = client.clone();
    let s_agent = agent_system.clone();
    let send = Callback::new(move |_: ()| {
        send_message(
            s_input.clone(), s_exec.clone(), s_attach.clone(),
            s_think.clone(), s_msgs.clone(), s_convs.clone(),
            s_sel.clone(), s_art.clone(), s_wi.clone(),
            s_abort.clone(), s_first,
            s_db.clone(), s_client.clone(), s_agent.clone(),
        )
    });

    let stop = Callback::new(move |_: ()| {
        if let Some(handle) = active_request_abort_handle.take() { handle.abort(); }
        is_local_executing.set(false);
        if let Some(conv_id) = selected_conversation_id() {
            let d = db_for_stop.clone();
            spawn(async move { let _ = backend::cancel_latest_execution_run_for_conversation(&d, conv_id).await; });
        }
    });

    ChatState { input_text, is_local_executing, pending_attachments, is_thinking, send, stop }
}

fn send_message(
    mut input_text: Signal<String>, mut is_local_executing: Signal<bool>,
    mut pending_attachments: Signal<Vec<(String, String, Vec<u8>)>>,
    mut is_thinking: Signal<bool>, mut messages_resource: Resource<Vec<backend::Message>>,
    mut conversations_resource: Resource<Vec<backend::Conversation>>,
    mut selected_conversation_id: Signal<Option<i64>>,
    mut selected_artifacts: Signal<Vec<backend::Artifact>>,
    mut selected_work_item: Signal<Option<backend::WorkItem>>,
    mut active_request_abort_handle: Signal<Option<tokio::task::JoinHandle<()>>>,
    is_first_msg_val: bool,
    db: std::sync::Arc<crate::db::Database>,
    client: std::sync::Arc<dyn crate::backend::client::BackendClient + Send + Sync>,
    agent: std::sync::Arc<crate::engine::system::AgentSystem>,
) {
    let val = input_text();
    let artifacts_for_prompt = selected_artifacts();
    let work_item_for_prompt = selected_work_item();

    let query_with_context = build_query_with_context(&val, &artifacts_for_prompt, work_item_for_prompt.as_ref());
    let has_attachments = !pending_attachments().is_empty();

    let db_for_msg = db.clone();
    let q = query_with_context.clone();
    is_local_executing.set(true);
    is_thinking.set(true);
    input_text.set(String::new());

    spawn(async move {
        let mut resolved_conv_id = selected_conversation_id();
        let mut created_new_conversation = false;

        if resolved_conv_id.is_none() {
            match backend::add_conversation(&db_for_msg, "New Conversation").await {
                Ok(id) => { created_new_conversation = true; resolved_conv_id = Some(id); selected_conversation_id.set(Some(id)); }
                Err(_) => { is_local_executing.set(false); is_thinking.set(false); return; }
            }
        }

        let conv_id = resolved_conv_id.unwrap();
        let (token_tx, mut token_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let mut partial = String::new();

        let _consumer = spawn(async move {
            while let Some(chunk) = token_rx.recv().await {
                partial.push_str(&chunk);
                messages_resource.restart();
            }
        });

        let spawn_llm_refinement = is_first_msg_val || created_new_conversation;

        // Process attachments
        if has_attachments {
            let att_list = pending_attachments();
            for (name, content_type, data) in att_list.iter() {
                let att_dir = crate::engine::utils::discovery::project_root().join("attachments").join(conv_id.to_string());
                let _ = std::fs::create_dir_all(&att_dir);
                let file_name = format!("{}_{}", chrono::Utc::now().timestamp_millis(), name);
                let file_path = att_dir.join(&file_name);
                if std::fs::write(&file_path, data).is_ok() {
                    let _ = backend::add_attachment(&db_for_msg, 0, name, &file_path.to_string_lossy(), content_type, data.len() as i64).await;
                }
            }
            pending_attachments.set(Vec::new());
        }

        let _ = backend::add_message(&db_for_msg, conv_id, "user", &query_with_context, false).await;

        let agent_ref = agent.clone();
        let db_for_orch = db.clone();
        let q_for_orch = q.clone();
        let handle = tokio::spawn(async move {
            let response = agent_ref.orchestrate(&db_for_orch, conv_id, &q_for_orch, Some(token_tx)).await;
            let _ = backend::add_message(&db_for_orch, conv_id, "assistant", &response, false).await;
        });

        if spawn_llm_refinement {
            let agent_title = agent.clone();
            let db_t = db.clone();
            let q_t = q.clone();
            let conv_id_t = conv_id;
            let (title_tx, mut title_rx) = tokio::sync::oneshot::channel::<()>();
            let mut convs_for_title = conversations_resource.clone();
            spawn(async move {
                if title_rx.await.is_ok() {
                    convs_for_title.restart();
                }
            });
            tokio::spawn(async move {
                let title = agent_title.generate_title(&db_t, &q_t).await;
                if !title.is_empty() {
                    let _ = crate::backend::update_conversation_title(&db_t, conv_id_t, &title).await;
                    let _ = title_tx.send(());
                    crate::engine::activity::engine_activity().bump();
                }
            });
        }

        if let Err(_) = handle.await {
            // orchestrate cancelled
        }
        messages_resource.restart();
        conversations_resource.restart();
        is_local_executing.set(false);
        is_thinking.set(false);
        selected_artifacts.set(Vec::new());
        selected_work_item.set(None);
    });
}
