// src/routes/conversations/chip.rs
use dioxus::prelude::*;

#[component]
pub fn ChatScreenChip(tone: String, label: String) -> Element {
    let class = match tone.as_str() {
        "blue" => "bg-blue-50 text-blue-700 border-blue-200",
        "violet" => "bg-violet-50 text-violet-700 border-violet-200",
        "green" => "bg-emerald-50 text-emerald-700 border-emerald-200",
        "red" => "bg-red-50 text-red-700 border-red-200",
        _ => "bg-slate-100 text-slate-600 border-slate-200",
    };
    rsx! {
        span { class: format!("inline-flex items-center rounded-full border px-3 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{label}"
        }
    }
}
