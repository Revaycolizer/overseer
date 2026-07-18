use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn SummaryTile(title: String, value: String, hint: String, icon: String) -> Element {
    rsx! {
        div { class: "rounded-2xl bg-slate-50/70 px-4 py-4 space-y-2",
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-[10px] font-bold uppercase tracking-widest text-slate-400", "{title}" }
                MaterialIcon { name: "{icon}", size: 16 }
            }
            p { class: "text-base font-black text-slate-900 break-words", "{value}" }
            p { class: "text-xs text-slate-500 leading-relaxed", "{hint}" }
        }
    }
}

#[component]
pub fn ChecklistItem(label: String, done: bool, help: String) -> Element {
    rsx! {
        div { class: "flex items-start gap-3",
            div {
                class: format!(
                    "mt-0.5 flex h-5 w-5 items-center justify-center rounded-full border {}",
                    if done {
                        "border-emerald-200 bg-emerald-50 text-emerald-600"
                    } else {
                        "border-amber-200 bg-amber-50 text-amber-700"
                    }
                ),
                MaterialIcon {
                    name: if done { "check" } else { "priority_high" },
                    size: 14
                }
            }
            div { class: "space-y-1",
                p { class: "text-sm font-semibold text-slate-900", "{label}" }
                p { class: "text-xs leading-relaxed text-slate-500", "{help}" }
            }
        }
    }
}

#[component]
pub fn HealthCard(
    title: String,
    status: String,
    good: bool,
    body: String,
    code: Option<String>,
    icon: String,
) -> Element {
    rsx! {
        div { class: "p-5 rounded-2xl bg-white/80 space-y-3",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    MaterialIcon { name: "{icon}", size: 16 }
                    p { class: "text-sm font-bold text-slate-900", "{title}" }
                }
                span {
                    class: format!(
                        "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                        if good {
                            "bg-emerald-50 text-emerald-600"
                        } else if status == "missing" {
                            "bg-amber-50 text-amber-700"
                        } else {
                            "bg-red-50 text-red-600"
                        }
                    ),
                    "{status}"
                }
            }
            p { class: "text-xs leading-relaxed text-slate-500", "{body}" }
            if let Some(code_value) = code {
                code { class: "block rounded-xl bg-slate-900/[0.035] px-3.5 py-2.5 text-xs font-medium tracking-[0.01em] text-slate-700 break-all font-mono", "{code_value}" }
            }
        }
    }
}
