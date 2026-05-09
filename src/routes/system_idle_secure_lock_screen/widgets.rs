use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn Widgets() -> Element {
    let mut efficiency = use_signal(|| 98u8);
    let mut temp = use_signal(|| 21i8);

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            // Use basic random with modulo for simplicity and compatibility
            let eff_val = (rand::random::<u8>() % 6) + 94; // 94-99
            let temp_val = (rand::random::<i8>() % 7).abs() + 18; // 18-24
            efficiency.set(eff_val);
            temp.set(temp_val);
        }
    });

    rsx! {
        div { class: "absolute bottom-12 left-1/2 -translate-x-1/2 flex gap-6 w-full max-w-5xl px-8 z-10",
            Widget {
                icon: "partly_cloudy_day",
                label: "Weather",
                value: format!("{temp}°C"),
                sub: "Scattered Clouds".to_string()
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
                sub: format!("{efficiency}%")
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
