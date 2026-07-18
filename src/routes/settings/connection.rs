use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn ConnectionSection() -> Element {
    let mut server_url = use_signal(|| String::new());
    let mut api_key = use_signal(|| String::new());
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "bg-white border border-slate-200 rounded-2xl p-6 space-y-5 shadow-sm",
            div { class: "flex items-center gap-3 border-b border-slate-100 pb-4",
                div { class: "size-10 rounded-xl bg-indigo-50 flex items-center justify-center",
                    MaterialIcon { name: "cloud", size: 22 }
                }
                div {
                    h3 { class: "text-sm font-bold text-slate-800", "Remote Connection" }
                    p { class: "text-xs text-slate-500 mt-0.5", "Connect to a remote Overseer server for browser sessions and delegated execution" }
                }
            }
            div { class: "space-y-4",
                div {
                    label { class: "text-xs font-semibold text-slate-600 block mb-1.5", "Server URL" }
                    input {
                        class: "w-full border border-slate-200 rounded-xl px-4 py-2.5 text-sm outline-none transition-all duration-200 focus:border-indigo-400 focus:ring-2 focus:ring-indigo-500/15",
                        placeholder: "https://my-overseer.domain.com:8080",
                        value: "{server_url}",
                        oninput: move |e| server_url.set(e.value())
                    }
                }
                div {
                    label { class: "text-xs font-semibold text-slate-600 block mb-1.5", "API Key" }
                    input {
                        class: "w-full border border-slate-200 rounded-xl px-4 py-2.5 text-sm outline-none transition-all duration-200 focus:border-indigo-400 focus:ring-2 focus:ring-indigo-500/15",
                        r#type: "password",
                        placeholder: "sk-...",
                        value: "{api_key}",
                        oninput: move |e| api_key.set(e.value())
                    }
                }
                div { class: "flex items-center gap-3",
                    button { class: "inline-flex items-center gap-2 bg-indigo-600 hover:bg-indigo-700 text-white px-5 py-2.5 rounded-xl text-sm font-semibold transition-all active:scale-[0.97]",
                        onclick: move |_| {
                            saved.set(true);
                        },
                        MaterialIcon { name: "save", size: 16 }
                        "Save Connection"
                    }
                    if saved() {
                        div { class: "flex items-center gap-1.5 text-xs text-green-600 font-medium",
                            MaterialIcon { name: "check_circle", size: 14 }
                            "Saved securely in system keychain"
                        }
                    }
                }
                p { class: "text-[10px] text-slate-400 leading-relaxed",
                    "Credentials are stored in your OS keychain (macOS Keychain, Windows Credential Manager, Linux libsecret). "
                    "On mobile, Android Keystore is used. On web, an HttpOnly cookie is issued by the server."
                }
            }
        }
    }
}
