use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub image_url: Option<String>,
}

#[component]
pub fn SearchBlock(payload: String, hide_evidence: bool) -> Element {
    if hide_evidence { return rsx! {}; }
    let results = serde_json::from_str::<Vec<SearchResult>>(&payload).unwrap_or_default();

    if results.is_empty() {
        return rsx! {
            div { class: "p-4 text-slate-400 text-xs italic bg-slate-50 rounded-xl border border-slate-100", "No structured results found." }
        };
    }

    rsx! {
        div { class: "w-full space-y-3",
            div { class: "flex items-center gap-2 px-1",
                MaterialIcon { name: "travel_explore", size: 16, color: MaterialIconColor::Custom("text-blue-500".to_string()) }
                span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-widest", "Intelligence Report" }
            }
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-3",
                for res in results {
                    a { href: "{res.url}", target: "_blank", class: "group flex flex-col bg-white rounded-xl border border-slate-100 shadow-sm hover:shadow-md hover:border-blue-200 transition-all overflow-hidden",
                        div { class: "relative h-32 bg-slate-50 flex items-center justify-center overflow-hidden border-b border-slate-50",
                            if let Some(img) = res.image_url {
                                img { src: "{img}", class: "w-full h-full object-cover transition-transform group-hover:scale-105" }
                            } else {
                                MaterialIcon { name: "public", size: 48, color: MaterialIconColor::Custom("text-slate-200".to_string()) }
                            }
                            div { class: "absolute bottom-2 left-2 px-2 py-0.5 bg-black/60 backdrop-blur-sm rounded text-[9px] text-white font-medium",
                                {res.url.split('/').nth(2).unwrap_or("Link")}
                            }
                        }
                        div { class: "p-3 space-y-1.5",
                            h3 { class: "text-xs font-bold text-slate-800 line-clamp-2 group-hover:text-blue-600 transition-colors", "{res.title}" }
                            p { class: "text-[10px] text-slate-500 line-clamp-3 leading-relaxed", "{res.snippet}" }
                        }
                    }
                }
            }
            div { class: "text-[9px] text-slate-300 font-medium text-right px-1", "Aggregated via Stealth Research" }
        }
    }
}
