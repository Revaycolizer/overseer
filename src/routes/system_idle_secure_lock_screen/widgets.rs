use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use sysinfo::System;

#[component]
pub fn Widgets() -> Element {
    let mut efficiency = use_signal(|| "Calculating...".to_string());
    let mut weather = use_signal(|| "Syncing...".to_string());
    let mut condition = use_signal(|| "Detecting...".to_string());

    // Weather Update
    use_future(move || async move {
        loop {
            // Fetch real weather from wttr.in (IP-based)
            if let Ok(response) = reqwest::get("https://wttr.in/?format=%t|%C").await {
                if let Ok(text) = response.text().await {
                    let parts: Vec<&str> = text.split('|').collect();
                    if parts.len() == 2 {
                        weather.set(parts[0].trim().to_string());
                        condition.set(parts[1].trim().to_string());
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1800)).await; // 30 mins
        }
    });

    // Efficiency Update (Real System Stats)
    use_future(move || async move {
        let mut sys = System::new_all();
        loop {
            sys.refresh_cpu_usage();
            // Wait a bit for accurate reading
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            sys.refresh_cpu_usage();
            
            let cpu_load = sys.global_cpu_usage();
            let eff = 100.0 - cpu_load;
            efficiency.set(format!("{:.0}%", eff));
            
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    rsx! {
        div { class: "absolute bottom-12 left-1/2 -translate-x-1/2 flex gap-6 w-full max-w-5xl px-8 z-10",
            Widget {
                icon: "partly_cloudy_day",
                label: "Weather",
                value: weather.read().clone(),
                sub: condition.read().clone()
            }
            Widget {
                icon: "calendar_today",
                label: "Next Meeting",
                value: "16:30".to_string(),
                sub: "Project Sync".to_string()
            }
            Widget {
                icon: "bolt",
                label: "Efficiency",
                value: "Optimal".to_string(),
                sub: efficiency.read().clone()
            }
        }
    }
}

#[component]
fn Widget(icon: &'static str, label: String, value: String, sub: String) -> Element {
    rsx! {
        div { class: "flex-1 bg-white/60 backdrop-blur-md p-6 rounded-2xl border border-slate-100 flex items-center gap-5 shadow-lg shadow-slate-200/40",
            div { class: "w-10 h-10 rounded-xl bg-slate-50 flex items-center justify-center border border-slate-100",
                MaterialIcon { name: icon, size: 20, color: dioxus_material_icons::MaterialIconColor::Custom("#64748b".into()) }
            }
            div { class: "text-left",
                p { class: "text-[10px] font-bold uppercase tracking-widest text-slate-400 mb-0.5", "{label}" }
                div { class: "flex items-baseline gap-1.5",
                    p { class: "text-lg font-bold text-slate-800", "{value}" }
                    span { class: "text-xs font-medium text-slate-400", "/ {sub}" }
                }
            }
        }
    }
}
