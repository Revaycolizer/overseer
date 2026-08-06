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
    drop(global_read);

    let input_text: Signal<String> = use_signal(String::new);
    let is_local_executing = use_signal(|| false);
    let pending_attachments = use_signal(Vec::<(String, String, Vec<u8>)>::new);
    // Stores the abort handle of the in-flight orchestration task so Stop works.
    let active_request_abort_handle = use_signal(|| None::<tokio::task::AbortHandle>);

    let messages_resource = props.messages_resource;
    let conversations_resource = props.conversations_resource;
    let is_thinking = props.is_thinking;
    let selected_conversation_id = props.selected_conversation_id;
    let selected_artifacts = props.selected_artifacts;
    let selected_work_item = props.selected_work_item;
    let mut starter_prompt = props.starter_prompt;
    let is_first_msg = props.is_first_msg;

    let mut input_text_mut = input_text;
    if let Some(prompt) = starter_prompt() {
        input_text_mut.set(prompt);
        starter_prompt.set(None);
    }

    let send_db = db.clone();
    let send_agent = agent_system.clone();
    let send = Callback::new(move |_: ()| {
        send_message(
            input_text,
            is_local_executing,
            pending_attachments,
            is_thinking,
            messages_resource,
            conversations_resource,
            selected_conversation_id,
            selected_artifacts,
            selected_work_item,
            active_request_abort_handle,
            is_first_msg,
            send_db.clone(),
            send_agent.clone(),
        )
    });

    let stop_db = db.clone();
    let stop = Callback::new(move |_: ()| {
        let mut is_local_executing = is_local_executing;
        let mut is_thinking = is_thinking;
        let mut active_request_abort_handle = active_request_abort_handle;
        if let Some(handle) = active_request_abort_handle.take() {
            handle.abort();
        }
        is_local_executing.set(false);
        is_thinking.set(false);
        if let Some(conv_id) = selected_conversation_id() {
            let d = stop_db.clone();
            spawn(async move {
                let _ = backend::cancel_latest_execution_run_for_conversation(&d, conv_id).await;
            });
        }
    });

    ChatState {
        input_text,
        is_local_executing,
        pending_attachments,
        is_thinking,
        send,
        stop,
    }
}

/// Persist the user message, create the execution run, and drive orchestration.
///
/// The user message persistence AND `AgentSystem::orchestrate` run together in a
/// single top-level `tokio` task whose abort handle is stored so Stop can cancel
/// it. Orchestration goes through the normal engine path, so every proposed tool
/// action is still governed by `overseer-kernel`. `is_thinking` /
/// `is_local_executing` are set before work begins and cleared on every
/// success / error / cancel path; failures render an assistant-visible error.
#[allow(clippy::too_many_arguments)]
fn send_message(
    mut input_text: Signal<String>,
    mut is_local_executing: Signal<bool>,
    mut pending_attachments: Signal<Vec<(String, String, Vec<u8>)>>,
    mut is_thinking: Signal<bool>,
    mut messages_resource: Resource<Vec<backend::Message>>,
    mut conversations_resource: Resource<Vec<backend::Conversation>>,
    mut selected_conversation_id: Signal<Option<i64>>,
    mut selected_artifacts: Signal<Vec<backend::Artifact>>,
    mut selected_work_item: Signal<Option<backend::WorkItem>>,
    mut active_request_abort_handle: Signal<Option<tokio::task::AbortHandle>>,
    is_first_msg_val: bool,
    db: std::sync::Arc<crate::db::Database>,
    agent: std::sync::Arc<crate::engine::system::AgentSystem>,
) {
    let val = input_text();
    if val.trim().is_empty() {
        return;
    }
    let artifacts_for_prompt = selected_artifacts();
    let work_item_for_prompt = selected_work_item();
    let query_with_context =
        build_query_with_context(&val, &artifacts_for_prompt, work_item_for_prompt.as_ref());
    let attachments = pending_attachments();

    // Enter the "thinking" state before any work begins.
    is_local_executing.set(true);
    is_thinking.set(true);
    input_text.set(String::new());
    pending_attachments.set(Vec::new());

    spawn(async move {
        // 1. Resolve (or create) the conversation.
        let mut resolved_conv_id = selected_conversation_id();
        let mut created_new_conversation = false;
        if resolved_conv_id.is_none() {
            match backend::add_conversation(&db, "New Conversation").await {
                Ok(id) => {
                    created_new_conversation = true;
                    resolved_conv_id = Some(id);
                    selected_conversation_id.set(Some(id));
                    conversations_resource.restart();
                }
                Err(_) => {
                    is_local_executing.set(false);
                    is_thinking.set(false);
                    return;
                }
            }
        }
        let conv_id = resolved_conv_id.unwrap();
        let spawn_llm_refinement = is_first_msg_val || created_new_conversation;

        // 2. Live token streaming: a Dioxus task drains the channel and refreshes
        //    the message list so the assistant reply renders as it arrives.
        let (token_tx, mut token_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let mut messages_for_stream = messages_resource;
        spawn(async move {
            while token_rx.recv().await.is_some() {
                messages_for_stream.restart();
            }
        });

        // 3. Orchestration task (Send-only values). Persists the user message,
        //    creates the execution run (inside orchestrate), routes tool actions
        //    through the kernel, and persists the assistant reply.
        let db_task = db.clone();
        let agent_task = agent.clone();
        let query_task = query_with_context.clone();
        let handle = tokio::spawn(async move {
            let msg_id = backend::add_message(&db_task, conv_id, "user", &query_task, false).await;

            // Persist any attachments against the stored user message.
            if let Ok(msg_id) = msg_id {
                for (name, mime, data) in attachments {
                    let dir = crate::db::get_overseer_data_dir().join("overseer_files");
                    let _ = std::fs::create_dir_all(&dir);
                    let path = dir.join(&name);
                    if std::fs::write(&path, &data).is_ok() {
                        let _ = backend::add_attachment(
                            &db_task,
                            msg_id,
                            &name,
                            &path.to_string_lossy(),
                            &mime,
                            data.len() as i64,
                        )
                        .await;
                    }
                }
            }

            if spawn_llm_refinement {
                let db_title = db_task.clone();
                let agent_title = agent_task.clone();
                let query_title = query_task.clone();
                tokio::spawn(async move {
                    let title = agent_title.generate_title(&db_title, &query_title).await;
                    if !title.is_empty() {
                        let _ =
                            backend::update_conversation_title(&db_title, conv_id, &title).await;
                    }
                });
            }

            // Deterministic governed commands (/write, /open, /screenshot) run
            // straight through the kernel — no model needed. Everything else goes
            // to the orchestrator.
            let response = if let Some(cmd) =
                crate::engine::governed_commands::parse_governed(&query_task)
            {
                crate::engine::governed_commands::run_governed(&db_task, conv_id, cmd).await
            } else {
                agent_task
                    .orchestrate(&db_task, conv_id, &query_task, Some(token_tx))
                    .await
            };
            let _ = backend::add_message(&db_task, conv_id, "assistant", &response, false).await;
            response
        });

        // 4. Store the abort handle so Stop can cancel this exact task.
        active_request_abort_handle.set(Some(handle.abort_handle()));

        // 5. Await completion; surface failures as an assistant-visible message.
        match handle.await {
            Ok(_response) => {}
            Err(join_err) if join_err.is_cancelled() => {
                let _ = backend::add_message(
                    &db,
                    conv_id,
                    "assistant",
                    "⏹️ Request cancelled.",
                    false,
                )
                .await;
            }
            Err(_) => {
                let _ = backend::add_message(
                    &db,
                    conv_id,
                    "assistant",
                    "⚠️ The request failed unexpectedly. Please try again.",
                    false,
                )
                .await;
            }
        }

        // 6. Clear state on every path.
        active_request_abort_handle.set(None);
        is_local_executing.set(false);
        is_thinking.set(false);
        messages_resource.restart();
        conversations_resource.restart();
        selected_artifacts.set(Vec::new());
        selected_work_item.set(None);
    });
}
