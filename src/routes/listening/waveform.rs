use dioxus::prelude::*;

#[component]
pub fn Waveform() -> Element {
    let bars = vec![
        ("0.1s", "40%"), ("0.4s", "70%"), ("0.2s", "30%"), ("0.6s", "90%"),
        ("0.3s", "50%"), ("0.8s", "20%"), ("0.5s", "60%"), ("0.1s", "80%"),
        ("0.7s", "40%"), ("0.4s", "70%"), ("0.2s", "30%"), ("0.6s", "90%"),
        ("0.3s", "50%"), ("0.8s", "20%"), ("0.5s", "60%"), ("0.1s", "80%"),
        ("0.7s", "40%"), ("0.4s", "70%"), ("0.2s", "30%"), ("0.6s", "90%"),
        ("0.3s", "50%"), ("0.8s", "20%"), ("0.5s", "60%"), ("0.1s", "80%"),
    ];

    rsx! {
        div { class: "w-full bg-surface-container-low rounded-xl p-4 flex items-center gap-1 h-16 border border-outline-variant/10 overflow-hidden",
            div { class: "waveform-container flex-1",
                for (delay , height) in bars {
                    div {
                        class: "waveform-bar",
                        style: "animation-delay: {delay}; height: {height};"
                    }
                }
            }
        }
    }
}
