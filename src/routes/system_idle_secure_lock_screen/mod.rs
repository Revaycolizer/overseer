use dioxus::prelude::*;
use dioxus_desktop::use_window;

mod clock;
mod status_panel;
mod widgets;

use clock::Clock;
use status_panel::StatusPanel;
use widgets::Widgets;

#[component]
pub fn SystemIdleSecureLockScreen() -> Element {
    let window = use_window();
    
    use_effect({
        let window = window.clone();
        move || {
            window.set_fullscreen(true);
        }
    });

    use_drop({
        let window = window.clone();
        move || {
            window.set_fullscreen(false);
        }
    });

    rsx! {
        main { class: "relative h-screen w-screen flex flex-col items-center justify-center bg-white overflow-hidden",
            // Decorative background concentric circles (Light Mode)
            div { class: "absolute inset-0 z-0 opacity-40 pointer-events-none flex items-center justify-center",
                div { class: "absolute w-[1400px] h-[1400px] border border-slate-100 rounded-full" }
                div { class: "absolute w-[1100px] h-[1100px] border border-slate-100 rounded-full" }
                div { class: "absolute w-[800px] h-[800px] border border-slate-100 rounded-full" }
                div { class: "absolute w-[500px] h-[500px] border border-slate-100 rounded-full" }
            }

            div { class: "relative z-10 flex flex-col items-center text-center space-y-16",
                Clock {}
                StatusPanel {}
            }

            Widgets {}

            // Decorative identifiers (Bottom Right)
            div { class: "absolute bottom-12 right-12 flex flex-col items-end gap-1 opacity-20 select-none",
                p { class: "text-[10px] font-bold tracking-[0.2em] text-slate-400 uppercase", "IDLE_STATE_ACTIVE" }
                p { class: "text-[10px] font-medium tracking-[0.1em] text-slate-400 uppercase", "LOC: 51.5074° N, 0.1278° W" }
            }
        }
    }
}


