use dioxus::prelude::*;
use dioxus_desktop::use_window;

mod header;
mod log_list;
mod monitor_sidebar;
mod waveform;
pub mod speech;

use header::Header;
use log_list::LogList;
use monitor_sidebar::MonitorSidebar;

#[component]
pub fn Listening() -> Element {
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
        main { class: "flex-1 flex overflow-hidden",
            div { class: "flex-1 flex flex-col min-w-0 bg-surface",
                Header {}
                LogList {}
            }
            MonitorSidebar {}
        }
    }
}
