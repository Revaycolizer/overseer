use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn LoginScreen() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut is_register = use_signal(|| false);

    rsx! {
        div { class: "flex min-h-screen items-center justify-center bg-slate-50 p-6",
            div { class: "w-full max-w-sm space-y-8",
                div { class: "text-center space-y-3",
                    div { class: "mx-auto size-14 rounded-2xl bg-blue-600 flex items-center justify-center shadow-[0_4px_12px_rgba(59,130,246,0.3)]",
                        MaterialIcon { name: "psychology", size: 30 }
                    }
                    h1 { class: "text-2xl font-extrabold text-slate-900 tracking-tight", "Overseer" }
                    p { class: "text-sm text-slate-500", if is_register() { "Create your account" } else { "Sign in to continue" } }
                }
                div { class: "bg-white border border-slate-200 rounded-2xl p-6 space-y-4 shadow-sm",
                    div { class: "space-y-3",
                        input { class: "w-full rounded-xl border border-slate-200 px-4 py-2.5 text-sm text-slate-900 placeholder:text-slate-400 outline-none transition-all duration-200 focus:border-blue-400 focus:ring-2 focus:ring-blue-500/15",
                            placeholder: "Username", value: "{username}", oninput: move |e| username.set(e.value())
                        }
                        input { class: "w-full rounded-xl border border-slate-200 px-4 py-2.5 text-sm text-slate-900 placeholder:text-slate-400 outline-none transition-all duration-200 focus:border-blue-400 focus:ring-2 focus:ring-blue-500/15",
                            r#type: "password", placeholder: "Password", value: "{password}", oninput: move |e| password.set(e.value())
                        }
                    }
                    if !error().is_empty() {
                        div { class: "rounded-xl bg-red-50 border border-red-200 px-4 py-3 text-xs text-red-700", "{error}" }
                    }
                    button { class: "w-full rounded-xl bg-blue-600 hover:bg-blue-700 text-white font-bold py-2.5 text-sm transition-all duration-200 ease-out active:scale-[0.97] shadow-[0_2px_8px_rgba(59,130,246,0.25)] hover:shadow-[0_4px_12px_rgba(59,130,246,0.35)]",
                        onclick: move |_| { error.set("Server connection required".into()); },
                        if is_register() { "Create Account" } else { "Sign In" }
                    }
                    button { class: "w-full text-xs text-slate-400 hover:text-slate-600 transition-colors duration-200 text-center cursor-pointer font-medium",
                        onclick: move |_| is_register.set(!is_register()),
                        if is_register() { "Already have an account? Sign in" } else { "No account? Create one" }
                    }
                }
            }
        }
    }
}
