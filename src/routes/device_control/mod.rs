use dioxus::prelude::*;

mod execution_log;
mod focal_viewport;
mod supervisor_panel;
mod tool_dock;

use execution_log::ExecutionLog;
use focal_viewport::FocalViewport;
use supervisor_panel::SupervisorPanel;
use tool_dock::ToolDock;

#[component]
pub fn DeviceControl() -> Element {
    rsx! {
        main { class: "flex-1 flex overflow-hidden",
            ExecutionLog {}
            div { class: "flex-1 flex flex-col bg-slate-100 overflow-hidden relative",
                FocalViewport {}
                ToolDock {}
            }
            SupervisorPanel {}
        }
    }
}
