use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub(crate) fn SystemLoadWidget() -> Element {
    rsx! { div { class: "bg-white border border-slate-200 rounded-xl p-4 space-y-2 shadow-sm",
        div { class: "flex items-center gap-2", MaterialIcon { name: "memory", size: 16 } span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "System Load" } }
        div { class: "flex items-end gap-2", span { class: "text-2xl font-black text-slate-800 font-mono", "43%" } span { class: "text-[10px] text-slate-400 font-medium mb-1", "+2.1% from baseline" } }
        div { class: "h-1.5 w-full bg-slate-100 rounded-full overflow-hidden", div { class: "h-full bg-gradient-to-r from-blue-500 to-indigo-500 rounded-full", style: "width: 43%" } }
    } }
}

#[component]
pub(crate) fn GpuPerformanceWidget() -> Element {
    rsx! { div { class: "bg-white border border-slate-200 rounded-xl p-4 space-y-2 shadow-sm",
        div { class: "flex items-center gap-2", MaterialIcon { name: "speed", size: 16 } span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "GPU Performance" } }
        div { class: "flex items-end gap-2", span { class: "text-2xl font-black text-slate-800 font-mono", "1.2 GHz" } span { class: "text-[10px] text-slate-400 font-medium mb-1", "VRAM 2.1/6.0 GB" } }
        div { class: "h-1.5 w-full bg-slate-100 rounded-full overflow-hidden", div { class: "h-full bg-gradient-to-r from-purple-500 to-pink-500 rounded-full", style: "width: 35%" } }
    } }
}

#[component]
pub(crate) fn BgProcessesWidget() -> Element {
    rsx! { div { class: "bg-white border border-slate-200 rounded-xl p-4 space-y-2 shadow-sm",
        div { class: "flex items-center gap-2", MaterialIcon { name: "dns", size: 16 } span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "Background Processes" } }
        div { class: "grid grid-cols-3 gap-3 text-center",
            div { class: "bg-slate-50 rounded-lg p-2", p { class: "text-xs font-bold text-slate-800", "156" } p { class: "text-[8px] text-slate-400", "Running" } }
            div { class: "bg-slate-50 rounded-lg p-2", p { class: "text-xs font-bold text-green-600", "12" } p { class: "text-[8px] text-slate-400", "Sleeping" } }
            div { class: "bg-slate-50 rounded-lg p-2", p { class: "text-xs font-bold text-amber-600", "3" } p { class: "text-[8px] text-slate-400", "Stopped" } }
        }
    } }
}

#[component]
pub(crate) fn EnvWidget() -> Element {
    rsx! { div { class: "bg-white border border-slate-200 rounded-xl p-4 space-y-2 shadow-sm",
        div { class: "flex items-center gap-2", MaterialIcon { name: "settings_applications", size: 16 } span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "Environment" } }
        div { class: "space-y-1.5 text-[11px]",
            div { class: "flex justify-between", span { class: "text-slate-500", "OS" } span { class: "font-medium text-slate-800", "Ubuntu 24.04" } }
            div { class: "flex justify-between", span { class: "text-slate-500", "Kernel" } span { class: "font-medium text-slate-800", "6.8.0-45-generic" } }
            div { class: "flex justify-between", span { class: "text-slate-500", "Desktop" } span { class: "font-medium text-slate-800", "X11" } }
        }
    } }
}
