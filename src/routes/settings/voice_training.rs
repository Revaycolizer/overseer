use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use crate::Route;
use crate::engine::voice::training::{record_sample, save_personal_wakeword, VoiceSample};

#[component]
pub fn VoiceTraining() -> Element {
    let mut samples = use_signal(Vec::<VoiceSample>::new);
    let mut recording = use_signal(|| false);
    let mut current_step = use_signal(|| 0);
    let nav = use_navigator();
    let state = crate::state::use_global_state();

    let on_record = move |_| {
        if *recording.peek() { return; }
        recording.set(true);
        spawn({
            let mut state = state.clone();
            async move {
                // Run the heavy audio capture on a blocking thread to avoid freezing UI
                let result = tokio::task::spawn_blocking(move || {
                    record_sample()
                }).await;

                if let Ok(Ok(s)) = result {
                    samples.push(s);
                    let step = current_step.peek().clone() + 1;
                    current_step.set(step);
                    if step >= 5 {
                        let _ = save_personal_wakeword(samples.cloned(), "assets/voice/models/personal.rpw");
                        state.write().has_trained_voice.set(true);
                        crate::engine::voice::wake_word::restart_wake_word_listener();
                        nav.push(Route::Settings {});
                    }
                }
                recording.set(false);
            }
        });
    };

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-slate-50 items-center justify-center p-6",
            div { class: "max-w-md w-full bg-white rounded-3xl shadow-xl p-8 space-y-8 text-center",
                div { class: "space-y-2",
                    div { class: "mx-auto w-16 h-16 bg-blue-100 text-blue-600 rounded-2xl flex items-center justify-center mb-4", MaterialIcon { name: "mic", size: 32 } }
                    h2 { class: "text-2xl font-black text-slate-900", "Voice Calibration" }
                    p { class: "text-slate-500 text-sm", "Say \"Hello Overseer\" clearly to train your personal wake-word model." }
                }

                div { class: "flex justify-center gap-2",
                    for i in 0..5 {
                        div { 
                            class: format!(
                                "h-2 w-12 rounded-full transition-all duration-500 {}",
                                if i < *current_step.read() { "bg-emerald-500" }
                                else if i == *current_step.read() && *recording.read() { "bg-blue-500 animate-pulse" }
                                else { "bg-slate-200" }
                            )
                        }
                    }
                }

                div { class: "relative h-32 flex items-center justify-center bg-slate-50 rounded-2xl border-2 border-dashed border-slate-200",
                    if *recording.read() {
                        div { class: "flex items-center gap-1",
                            for i in 0..12 {
                                div { 
                                    class: "w-1 bg-blue-500 rounded-full animate-bounce",
                                    style: "height: {20 + (i % 3) * 10}px; animation-delay: {i as f32 * 0.1}s"
                                }
                            }
                        }
                    } else {
                        p { class: "text-slate-400 font-medium text-sm", "Ready for Sample {*current_step.read() + 1}" }
                    }
                }

                button {
                    class: format!(
                        "w-full py-4 rounded-2xl font-bold text-white transition-all transform active:scale-95 {}",
                        if *recording.read() { "bg-slate-400 cursor-not-allowed" } else { "bg-blue-600 hover:bg-blue-700 shadow-lg shadow-blue-200" }
                    ),
                    onclick: on_record,
                    disabled: *recording.read(),
                    if *recording.read() { "Recording..." } else { "Capture Sample {*current_step.read() + 1}" }
                }

                button {
                    class: "text-slate-400 hover:text-slate-600 text-xs font-bold uppercase tracking-widest",
                    onclick: move |_| { nav.push(Route::Settings {}); },
                    "Skip Calibration"
                }
            }
        }
    }
}
