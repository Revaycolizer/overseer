use dioxus::prelude::*;
use chrono::Local;

#[component]
pub fn Clock() -> Element {
    let mut now = use_signal(|| Local::now());

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            now.set(Local::now());
        }
    });

    let time_str = now.read().format("%H:%M").to_string();
    let date_str = now.read().format("%A, %B %e").to_string();

    rsx! {
        div { class: "space-y-4",
            h1 { class: "text-[15rem] font-extralight tracking-tighter text-slate-900 leading-none select-none",
                "{time_str}"
            }
            div { class: "flex items-center justify-center gap-4",
                span { class: "text-xs uppercase tracking-[0.3em] text-slate-400 font-bold", "{date_str}" }
                span { class: "w-1 h-1 bg-slate-200 rounded-full" }
                span { class: "text-xs uppercase tracking-[0.3em] text-slate-400 font-bold", "Overseer HQ" }
            }
        }
    }
}
