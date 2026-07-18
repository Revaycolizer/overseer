use dioxus::prelude::*;
use crate::backend;

use super::chat_input::ChatInput;
use super::empty_state::EmptyConversationState;
use super::helpers::copy_to_clipboard;
use super::message_bubble::MessageBubble;
use super::thinking_bubble::ThinkingBubble;

#[component]
pub fn ChatPanel(
    messages: Vec<backend::Message>,
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    conversations_resource: Resource<Vec<backend::Conversation>>,
    selected_conversation_id: Signal<Option<i64>>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    is_first_msg: bool,
    is_thinking: Signal<bool>,
    starter_prompt: Signal<Option<String>>,
    messages_has_more: Signal<bool>,
    messages_loading_more: Signal<bool>,
    load_more: Callback<()>,
) -> Element {
    let total_tokens = crate::engine::tokenizer::count_messages_tokens(&messages);
    rsx! {
        div {
            id: "chat-scroll-container",
            class: "flex-1 overflow-y-auto scroll-smooth custom-scrollbar",
            onscroll: move |evt| {
                if evt.scroll_top() < 100.0 && messages_has_more() && !messages_loading_more() {
                    load_more.call(());
                }
            },
            div { class: "w-full max-w-3xl mx-auto py-6 sm:py-10 px-3 sm:px-6 space-y-6 sm:space-y-8",
                if !messages.is_empty() {
                    div { class: "mx-auto text-center text-[10px] font-bold uppercase tracking-[0.18em] text-slate-400",
                        "~{total_tokens} tokens in view"
                    }
                }
                if messages.is_empty() && !is_thinking() {
                    EmptyConversationState { on_prompt: move |prompt: String| starter_prompt.set(Some(prompt)) }
                } else {
                    if messages_loading_more() {
                        div { class: "mx-auto text-center text-[11px] font-bold uppercase tracking-[0.18em] text-slate-400 py-2", "Loading older messages..." }
                    }
                    for msg in &messages { MessageBubble { msg: msg.clone(), on_copy: move |content: String| copy_to_clipboard(content) } }
                    if is_thinking() { ThinkingBubble {} }
                }
            }
        }
        ChatInput {
            selected_conversation_id: selected_conversation_id,
            messages_resource: messages_resource.clone(),
            visible_messages: visible_messages,
            conversations_resource: conversations_resource.clone(),
            selected_artifacts: selected_artifacts,
            selected_work_item: selected_work_item,
            is_first_msg: is_first_msg,
            is_thinking: is_thinking,
            starter_prompt: starter_prompt,
        }
    }
}