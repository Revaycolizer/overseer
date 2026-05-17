use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use super::waveform::Waveform;
use crate::state::use_global_state;

#[component]
pub fn Header() -> Element {
    let mut state = use_global_state();
    let transcript = state.read().voice_transcript.read().clone();
    let result = state.read().voice_result.read().clone();
    let mut input_text = use_signal(String::new);
    let nav = use_navigator();

    let (status_text, status_color) = if result.is_some() {
        ("Command Complete", "text-emerald-400")
    } else if transcript.is_some() {
        ("Processing Command...", "text-amber-400 animate-pulse")
    } else {
        ("Overseer Awaiting Command...", "text-blue-400")
    };

    let mut on_submit = move || {
        let cmd = input_text.peek().trim().to_string();
        if cmd.is_empty() { return; }
        input_text.set(String::new());
        state.write().voice_transcript.set(Some(cmd.clone()));
        
        let db = state.peek().db.clone();
        let agent = state.peek().agent_system.clone();
        spawn(async move {
            match crate::engine::voice::pipeline::execute_voice_command(&db, &agent, &cmd).await {
                Ok(res) => {
                    state.write().voice_result.set(Some(res));
                }
                Err(e) => {
                    state.write().voice_result.set(Some(format!("Execution Error: {}", e)));
                }
            }
        });
    };

    let on_cancel = move |_| {
        state.write().is_listening.set(false);
        state.write().is_inhibited.set(false);
        state.write().voice_transcript.set(None);
        state.write().voice_result.set(None);
        nav.push(crate::Route::Conversations {});
    };

    rsx! {
        section { class: "p-8 border-b border-white/5 bg-slate-900/90 backdrop-blur-md shadow-2xl relative",
            div { class: "flex items-center justify-between mb-6",
                div { class: "flex flex-col gap-1",
                    span { class: "text-[10px] uppercase tracking-[0.2em] font-extrabold {status_color}", "{status_text}" }
                    h1 { class: "text-3xl font-black tracking-tight text-white",
                        if let Some(ref t) = transcript { "\"{t}\"" } else { "System Active" }
                    }
                }
                button { 
                    class: "bg-red-500/10 text-red-400 border border-red-500/20 px-4 py-2.5 rounded-xl font-bold text-xs flex items-center gap-2 hover:bg-red-500 hover:text-white transition-all transform active:scale-95 shadow-lg",
                    onclick: on_cancel,
                    MaterialIcon { name: "close", size: 16 }
                    "Exit Immersive Screen"
                }
            }

            if transcript.is_none() {
                div { class: "space-y-6",
                    Waveform {}
                    div { class: "relative max-w-2xl mx-auto flex items-center bg-white/5 border border-white/10 rounded-2xl p-1.5 focus-within:border-blue-500/50 shadow-inner group transition-all duration-300",
                        input {
                            class: "flex-1 bg-transparent border-0 outline-none text-white text-base px-4 py-3 placeholder:text-slate-500",
                            placeholder: "Type a command (e.g. \"Take a screenshot\", \"Open Firefox\")...",
                            value: "{input_text}",
                            oninput: move |e| input_text.set(e.value()),
                            onkeydown: move |e| { if e.key() == Key::Enter { on_submit(); } }
                        }
                        button { 
                            class: "bg-blue-600 hover:bg-blue-700 text-white p-3 rounded-xl transition-all transform active:scale-95 shadow-md flex items-center justify-center",
                            onclick: move |_| on_submit(),
                            MaterialIcon { name: "arrow_forward", size: 20 }
                        }
                    }
                }
            }

            if let Some(ref response) = result {
                div { class: "mt-6 p-5 bg-white/5 rounded-2xl border border-white/10 text-sm text-slate-200 leading-relaxed max-h-56 overflow-y-auto shadow-inner custom-scrollbar font-medium",
                    "{response}"
                }
            }
        }
    }
}
