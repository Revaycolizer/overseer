use crate::Route;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

const BRAND_ICON: &str = "/assets/overseer.png";

#[component]
pub fn Sidebar() -> Element {
    let route = use_route::<Route>();

    rsx! {
        aside { class: "w-[64px] border-r border-slate-200 bg-white flex flex-col items-center py-4 shrink-0 shadow-sm",
            div { class: "mb-6",
                div { class: "p-1 flex items-center justify-center cursor-pointer hover:scale-105 transition-transform",
                    img {
                        src: BRAND_ICON,
                        alt: "Overseer logo",
                        class: "w-8 h-8 object-contain"
                    }
                }
            }
            nav { class: "flex-1 flex flex-col items-center gap-1",
                Link {
                    class: if route == (Route::Conversations {}) { "nav-link active" } else { "nav-link" },
                    to: Route::Conversations {},
                    title: "Chat",
                    MaterialIcon { name: "chat", size: 22 }
                }
                Link {
                    class: if route == (Route::AgentsWorkshop {}) { "nav-link active" } else { "nav-link" },
                    to: Route::AgentsWorkshop {},
                    title: "Agents",
                    MaterialIcon { name: "smart_toy", size: 22 }
                }
                Link {
                    class: if route == (Route::WorkflowAutomation {}) { "nav-link active" } else { "nav-link" },
                    to: Route::WorkflowAutomation {},
                    title: "Workflows",
                    MaterialIcon { name: "terminal", size: 22 }
                }
                Link {
                    class: if route == (Route::ToolsWorkshop {}) { "nav-link active" } else { "nav-link" },
                    to: Route::ToolsWorkshop {},
                    title: "Tools",
                    MaterialIcon { name: "build", size: 22 }
                }
                Link {
                    class: if route == (Route::ComputerControl {}) { "nav-link active" } else { "nav-link" },
                    to: Route::ComputerControl {},
                    title: "Computer Control",
                    MaterialIcon { name: "computer", size: 22 }
                }
                Link {
                    class: if route == (Route::SupervisorsWorkshop {}) { "nav-link active" } else { "nav-link" },
                    to: Route::SupervisorsWorkshop {},
                    title: "Supervisors",
                    MaterialIcon { name: "supervised_user_circle", size: 22 }
                }
                Link {
                    class: if route == (Route::PlannersWorkshop {}) { "nav-link active" } else { "nav-link" },
                    to: Route::PlannersWorkshop {},
                    title: "Planners",
                    MaterialIcon { name: "architecture", size: 22 }
                }
                Link {
                    class: if route == (Route::Settings {}) { "nav-link active" } else { "nav-link" },
                    to: Route::Settings {},
                    title: "Settings",
                    MaterialIcon { name: "settings", size: 22 }
                }
            }
            div { class: "mt-auto flex flex-col items-center gap-4",
                div { class: "h-8 w-8 rounded-full border border-slate-200 overflow-hidden shrink-0 cursor-pointer hover:ring-2 hover:ring-slate-200 transition-all",
                    img {
                        alt: "User profile",
                        class: "w-full h-full object-cover",
                        src: "https://lh3.googleusercontent.com/aida-public/AB6AXuCdJgFWwq2dLrKHJbTjZwb26-IzGtixjZB_bjNTmQZlWjnYzfc-bnEHi_PayiOb9_0aSDtoQZcGw2D86C3jJeXDtcwh3Xli-zBNWPPXOH7RzFOGBlx67RBqoAxQ_P4CGCEE8FB7_3twEJf4ZTAllayry-vF9VnzTBDz8L2uXg0Vu3YIN2zACdtMWgurvWuxDQbcPaXE5pGkvq2RBqBBDNT32oyayfpwzntqKyvQ9gheEDKBunBQTf4itKP_ZzqbI3JriaB1E4BVDpI",
                    }
                }
            }
        }
    }
}

#[component]
pub fn Layout() -> Element {
    let route = use_route::<Route>();
    #[cfg(feature = "desktop")]
    let window = dioxus::desktop::use_window();

    let title = match route {
        Route::Conversations {} => "Overseer",
        Route::AgentsWorkshop {} => "Agents Workshop",
        Route::ToolsWorkshop {} => "Tools Workshop",
        Route::ComputerControl {} => "Computer Control",
        Route::WorkflowAutomation {} => "Workflow Automation",
        Route::SupervisorsWorkshop {} => "Supervisors Workshop",
        Route::PlannersWorkshop {} => "Planners Workshop",
        Route::Settings {} => "Settings",
        _ => "Overseer",
    };

    rsx! {
        div { class: "window-frame h-screen bg-white flex flex-col border border-slate-300 overflow-hidden",
            div {
                class: "h-10 bg-[#f1f5f9] border-b border-slate-200 flex items-center justify-between px-4 shrink-0 select-none",
                onmousedown: {
                    #[cfg(feature = "desktop")]
                    let window = window.clone();
                    move |_| {
                        #[cfg(feature = "desktop")]
                        window.drag();
                    }
                },
                div { class: "flex items-center gap-4",
                    div { class: "flex gap-2 group/controls",
                        div {
                            class: "w-3.5 h-3.5 rounded-full bg-[#ff5f56] border border-[#e0443e] cursor-pointer flex items-center justify-center transition-all",
                            onclick: {
                                #[cfg(feature = "desktop")]
                                let window = window.clone();
                                move |e| {
                                    e.stop_propagation();
                                    #[cfg(feature = "desktop")]
                                    window.close();
                                }
                            },
                            onmousedown: |e| e.stop_propagation(),
                            span { class: "text-[10px] text-[#4c0000] opacity-0 group-hover/controls:opacity-100 transition-opacity font-bold select-none mb-[1px]", "x" }
                        }
                        div {
                            class: "w-3.5 h-3.5 rounded-full bg-[#ffbd2e] border border-[#dea123] cursor-pointer flex items-center justify-center transition-all",
                            onclick: {
                                #[cfg(feature = "desktop")]
                                let window = window.clone();
                                move |e| {
                                    e.stop_propagation();
                                    #[cfg(feature = "desktop")]
                                    window.set_minimized(true);
                                }
                            },
                            onmousedown: |e| e.stop_propagation(),
                            span { class: "text-[12px] text-[#9a6a00] opacity-0 group-hover/controls:opacity-100 transition-opacity font-bold select-none mb-[2px]", "-" }
                        }
                        div {
                            class: "w-3.5 h-3.5 rounded-full bg-[#27c93f] border border-[#1aab29] cursor-pointer flex items-center justify-center transition-all",
                            onclick: {
                                #[cfg(feature = "desktop")]
                                let window = window.clone();
                                move |e| {
                                    e.stop_propagation();
                                    #[cfg(feature = "desktop")]
                                    window.set_maximized(!window.is_maximized());
                                }
                            },
                            onmousedown: |e| e.stop_propagation(),
                            span { class: "text-[8px] text-[#006400] opacity-0 group-hover/controls:opacity-100 transition-opacity font-bold select-none", "+" }
                        }
                    }
                    div { class: "flex items-center gap-2 text-slate-500",
                        img {
                            src: BRAND_ICON,
                            alt: "Overseer logo",
                            class: "w-4 h-4 object-contain"
                        }
                        span { class: "text-xs font-semibold tracking-wide uppercase", "{title}" }
                    }
                }
                div { class: "flex items-center gap-3",
                    div { class: "h-4 w-[1px] bg-slate-200 mx-2" }
                    button { class: "p-1.5 rounded-md hover:bg-slate-200 transition-colors flex items-center justify-center",
                        MaterialIcon { name: "search", color: MaterialIconColor::Custom("text-slate-500".to_string()), size: 18 }
                    }
                    button { class: "p-1.5 rounded-md hover:bg-slate-200 transition-colors flex items-center justify-center",
                        MaterialIcon { name: "more_vert", color: MaterialIconColor::Custom("text-slate-500".to_string()), size: 18 }
                    }
                }
            }
            div { class: "flex flex-1 overflow-hidden",
                Sidebar {}
                div { class: "flex-1 flex flex-col min-w-0 min-h-0 bg-background-light overflow-hidden",
                    Outlet::<Route> {}
                }
            }
        }
    }
}
