use dioxus::prelude::*;

#[component]
pub fn AnimatedPlaceholder(selected_conversation_id: Signal<Option<i64>>) -> Element {
    let mut animated_text = use_signal(|| String::new());
    use_future(move || async move {
        let new_prompts = [
            "Type anything to start a new conversation...",
            "Ask a question, plan a task, or explore an idea...",
            "Drop a request and Overseer will route it...",
        ];
        let active_prompts = [
            "Ask Overseer to analyze, automate, or explain...",
            "Request a follow-up, summary, or next step...",
            "Refine this conversation with more context...",
        ];
        let mut phrase_index = 0usize;
        loop {
            let prompts = if selected_conversation_id().is_some() { &active_prompts } else { &new_prompts };
            let phrase = prompts[phrase_index % prompts.len()];
            let chars: Vec<char> = phrase.chars().collect();
            for i in 1..=chars.len() { animated_text.set(chars.iter().take(i).collect()); tokio::time::sleep(std::time::Duration::from_millis(45)).await; }
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            for i in (0..chars.len()).rev() { animated_text.set(chars.iter().take(i).collect()); tokio::time::sleep(std::time::Duration::from_millis(30)).await; }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            phrase_index = phrase_index.wrapping_add(1);
        }
    });
    rsx! { "{animated_text}" }
}
