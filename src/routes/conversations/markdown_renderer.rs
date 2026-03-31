use dioxus::prelude::*;
use crate::routes::conversations::helpers::clean_content;

#[component]
pub fn MarkdownText(content: String, #[props(default = false)] is_user: bool) -> Element {
    let cleaned = if is_user { content } else { clean_content(&content) };
    
    rsx! {
        div { class: "markdown-content text-sm leading-relaxed",
            for line in cleaned.lines() {
                p { class: "mb-2 last:mb-0", "{line}" }
            }
        }
    }
}
