// src/routes/conversations/prefetch.rs
use crate::backend;
use super::state::ConversationsState;
use dioxus::prelude::*;

const PAGE_SIZE: u32 = 50;

pub fn setup_prefetching_and_pagination(
    db: std::sync::Arc<crate::db::Database>,
    state: ConversationsState,
) -> impl Fn() {
    let selected_conversation_id = state.selected_conversation_id;
    let mut visible_messages = state.visible_messages;
    let mut messages_cursor = state.messages_cursor;
    let mut messages_has_more = state.messages_has_more;
    let messages_loading_more = state.messages_loading_more;

    let db_for_page = db.clone();
    use_effect(move || {
        let conv_id = selected_conversation_id();
        let db = db_for_page.clone();
        visible_messages.set(Vec::new());
        messages_cursor.set(None);
        messages_has_more.set(true);
        if let Some(id) = conv_id {
            if let Some(cached) = crate::engine::cache::messages().read().ok().and_then(|c| c.get(&id).cloned()) {
                messages_cursor.set(cached.last().map(|m| m.id));
                messages_has_more.set(cached.len() as u32 >= PAGE_SIZE);
                visible_messages.set(cached);
            } else {
                spawn(async move {
                    let page = backend::get_messages_page(&db, id, None, PAGE_SIZE).await.unwrap_or_default();
                    messages_cursor.set(page.last().map(|m| m.id));
                    messages_has_more.set(page.len() as u32 >= PAGE_SIZE);
                    if let Ok(mut cache) = crate::engine::cache::messages().write() {
                        cache.put(id, page.clone());
                    }
                    visible_messages.set(page);
                });
            }
        }
    });

    let db_for_load = db.clone();
    let load_more = move || {
        let mut visible_messages = visible_messages.clone();
        let mut messages_cursor = messages_cursor.clone();
        let mut messages_has_more = messages_has_more.clone();
        let mut messages_loading_more = messages_loading_more.clone();

        if messages_loading_more() || !messages_has_more() { return; }
        let Some(conv_id) = selected_conversation_id() else { return; };

        let cached_opt = crate::engine::cache::messages().read().ok().and_then(|c| c.get(&conv_id).cloned());
        if let Some(cached) = cached_opt {
            let (visible_len, visible_clone) = {
                let visible = visible_messages.read();
                (visible.len(), visible.iter().cloned().collect::<Vec<_>>())
            };
            if cached.len() > visible_len {
                let diff = cached.len() - visible_len;
                let take_count = diff.min(PAGE_SIZE as usize);
                let start_idx = cached.len() - visible_len - take_count;
                let end_idx = cached.len() - visible_len;
                let page = cached[start_idx..end_idx].to_vec();

                let mut combined = page.clone();
                combined.extend(visible_clone);
                visible_messages.set(combined);
                messages_cursor.set(page.last().map(|m| m.id));
                messages_has_more.set(start_idx > 0 || messages_has_more.peek().clone());
                return;
            }
        }

        messages_loading_more.set(true);
        let cursor = messages_cursor.read().clone();
        let db = db_for_load.clone();
        let mut visible_messages_clone = visible_messages.clone();
        let mut messages_cursor_clone = messages_cursor.clone();
        let mut messages_has_more_clone = messages_has_more.clone();
        let mut messages_loading_more_clone = messages_loading_more.clone();
        spawn(async move {
            let page = backend::get_messages_page(&db, conv_id, cursor, PAGE_SIZE).await.unwrap_or_default();
            if !page.is_empty() {
                let mut combined = page.clone();
                combined.extend(visible_messages_clone.read().iter().cloned());
                visible_messages_clone.set(combined);
                messages_cursor_clone.set(page.last().map(|m| m.id));
                messages_has_more_clone.set(page.len() as u32 >= PAGE_SIZE);
            }
            messages_loading_more_clone.set(false);
        });
    };

    let db_for_prefetch = db.clone();
    use_future(move || {
        let db = db_for_prefetch.clone();
        let conv_id = selected_conversation_id();
        async move {
            let Some(id) = conv_id else { return; };
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            for _ in 0..5 {
                let oldest_id = crate::engine::cache::messages().read().ok().and_then(|c| c.get(&id).and_then(|v| v.first().map(|m| m.id)));
                let page = backend::get_messages_page(&db, id, oldest_id, PAGE_SIZE).await.unwrap_or_default();
                if page.is_empty() { break; }

                if let Ok(mut cache) = crate::engine::cache::messages().write() {
                    if let Some(cached) = cache.get(&id).cloned() {
                        let mut combined = page.clone();
                        combined.extend(cached);
                        cache.put(id, combined);
                    } else {
                        cache.put(id, page.clone());
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    });

    load_more
}
