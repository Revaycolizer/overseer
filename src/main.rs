#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::prelude::*;
use dioxus_material_icons::MaterialIconStylesheet;
mod backend;
#[path = "db/mod.rs"]
pub mod db;
pub mod engine;
mod routes;
mod state;

use crate::db::Database;
use routes::{
    AgentsWorkshop, ComputerControl, Conversations, Layout, PlannersWorkshop, Settings,
    SupervisorsWorkshop, ToolsWorkshop, WorkflowAutomation, Listening, SystemIdleSecureLockScreen
};
use state::StateProvider;
use std::sync::Arc;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
    #[layout(IdleWatcher)]
    #[layout(Layout)]
    #[route("/")]
    Conversations {},
    #[route("/agents")]
    AgentsWorkshop {},
    #[route("/workflows")]
    WorkflowAutomation {},
    #[route("/tools")]
    ToolsWorkshop {},
    #[route("/computer")]
    ComputerControl {},
    #[route("/supervisors")]
    SupervisorsWorkshop {},
    #[route("/planners")]
    PlannersWorkshop {},
    #[route("/settings")]
    Settings {},
    #[end_layout]
    #[route("/listening")]
    Listening {},
    #[route("/idle")]
    SystemIdleSecureLockScreen {},
}

const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const ICON_PNG: Asset = asset!("/assets/overseer.png");

const PRISM_CSS: Asset = asset!("/assets/prism/prism-tomorrow.min.css");
const PRISM_BUNDLE_JS: Asset = asset!("/assets/prism/prism-bundle.min.js");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("📍 Overseer launched with args: {:?}", args);

    // Check if we should run in CLI mode
    if args.len() > 1 && (args[1] == "info" || args[1] == "--info" || !args[1].starts_with("-")) {
        println!("🖥️ Entering CLI mode...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(engine::shell::run_cli()) {
            eprintln!("Error: {}", e);
        }
        return;
    }

    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        let config = Config::new().with_window(
            WindowBuilder::new()
                .with_title("Overseer")
                .with_decorations(false),
        );
        dioxus::LaunchBuilder::desktop()
            .with_cfg(config)
            .launch(App);
    }
    #[cfg(not(feature = "desktop"))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    println!("🏗️  App component rendering...");
    let mut db = use_resource(|| async move {
        println!("🛠️  Database resource initialization started...");
        let result = Database::new().await;
        match &result {
            Ok(_) => println!("✅ Database resource initialization successful!"),
            Err(e) => eprintln!("❌ Database resource initialization failed: {}", e),
        }
        result.map_err(|e| format!("{:#}", e))
    });

    rsx! {
        document::Link { rel: "icon", href: ICON_PNG }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }

        // PrismJS for Syntax Highlighting
        document::Link { rel: "stylesheet", href: PRISM_CSS }
        document::Script { src: PRISM_BUNDLE_JS }

        // PrismJS for Syntax Highlighting
        document::Link { rel: "stylesheet", href: PRISM_CSS }
        document::Script { src: PRISM_BUNDLE_JS }

        document::Script {
            {r#"
            document.addEventListener('scroll', function(e) {
                if (e.target.classList && e.target.classList.contains('code-editor-textarea')) {
                    let pre = e.target.nextElementSibling;
                    if (pre && pre.tagName === 'PRE') {
                        pre.scrollTop = e.target.scrollTop;
                        pre.scrollLeft = e.target.scrollLeft;
                    }
                }
            }, true);
            "#}
        }

        MaterialIconStylesheet {}

        if let Some(res) = db.cloned() {
            match res {
                Ok(database) => rsx! {
                    StateProvider {
                        database: Arc::new(database),
                        Router::<Route> {}
                    }
                },
                Err(error_message) => rsx! {
                    div { class: "flex items-center justify-center h-screen bg-[#0f1116] text-white flex-col",
                        div { class: "p-8 bg-red-900/20 border border-red-500 rounded-lg max-w-2xl",
                            h2 { class: "text-xl font-bold text-red-500 mb-4", "Database Initialization Failed" }
                            p { class: "text-gray-300 mb-4", "The application encountered an error while setting up the local runtime. The message below includes the failing stage so it is easier to repair." }
                            pre { class: "bg-black/40 p-4 rounded text-xs overflow-auto max-h-64 text-red-300 whitespace-pre-wrap",
                                "{error_message}"
                            }
                            button {
                                class: "mt-6 px-4 py-2 bg-red-600 hover:bg-red-700 rounded transition-colors font-medium",
                                onclick: move |_| db.restart(),
                                "Retry Initialization"
                            }
                        }
                    }
                }
            }
        } else {
            div { class: "flex flex-col items-center justify-center h-screen bg-white/80 text-white overflow-hidden",
                // Animated background glow
                div { class: "absolute w-[500px] h-[500px] bg-blue-600/10 rounded-full blur-[120px] -z-10 translate-y-[-20%] animate-pulse" }

                // Elegant loading container
                div { class: "flex flex-col items-center gap-8",
                    // Custom sophisticated loader
                    div { class: "relative w-20 h-20",
                        div { class: "absolute inset-0 rounded-full border-4 border-slate-800" }
                        div { class: "absolute inset-0 rounded-full border-4 border-t-blue-500 border-r-transparent border-b-transparent border-l-transparent animate-spin" }
                        div { class: "absolute inset-2 rounded-full border-2 border-slate-800" }
                        div { class: "absolute inset-2 rounded-full border-2 border-b-blue-400 border-t-transparent border-r-transparent border-l-transparent animate-spin-reverse [animation-duration:1.5s]" }
                    }

                    div { class: "text-center space-y-3",
                        h1 { class: "text-4xl font-black tracking-tighter text-transparent bg-clip-text bg-gradient-to-br from-white to-slate-400",
                            "OVERSEER"
                        }
                        div { class: "flex flex-col items-center gap-2",
                            p { class: "text-sm font-bold text-blue-400 uppercase tracking-[0.2em] animate-pulse",
                                "Initializing System"
                            }
                            div { class: "w-48 h-1 bg-slate-900 rounded-full overflow-hidden",
                                div { class: "h-full bg-blue-600 animate-[loading-bar_2s_infinite_ease-in-out]", style: "width: 40%" }
                            }
                        }
                    }
                }

                // Version badge
                div { class: "absolute bottom-12 px-4 py-1.5 rounded-full border border-slate-800 bg-slate-900/50 text-[10px] font-bold text-slate-500 uppercase tracking-widest",
                    "Version 1.0.0 LTS"
                }
            }
        }
    }
}

#[component]
pub fn IdleWatcher() -> Element {
    let nav = use_navigator();
    let route = use_route::<Route>();
    let state = use_context::<Signal<crate::state::GlobalState>>();

    let timer_route = route.clone();
    let timer_nav = nav.clone();

    let mut idle_time = use_signal(|| 0u64);

    // Idle Timer Loop
    use_future(move || {
        let tn = timer_nav.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                
                let (last, inhibited) = {
                    let s = state.peek();
                    let last_val = *s.last_activity.peek();
                    let inhib_val = *s.is_inhibited.peek();
                    (last_val, inhib_val)
                };
                
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                idle_time.set(now - last);

                // For testing, we use 10 seconds. In production, this should be 300 (5 mins).
                if !inhibited && (now - last) >= 10 { 
                    tn.push(Route::SystemIdleSecureLockScreen {});
                }
            }
        }
    });

    let mut entered_idle_at = use_signal(|| 0u64);

    // Sync idle entry time without use_effect to ensure it's always up to date with the render cycle
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    if let Route::SystemIdleSecureLockScreen {} = route {
        if *entered_idle_at.peek() == 0 {
            entered_idle_at.set(now);
        }
    } else if *entered_idle_at.peek() != 0 {
        entered_idle_at.set(0);
    }

    rsx! {
        div { 
            class: "h-screen w-screen relative overflow-hidden",
            onmousedown: {
                let route = route.clone();
                let nav = nav.clone();
                let state = state.clone();
                move |_| {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let mut last_activity = state.peek().last_activity;
                    last_activity.set(now);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now - *entered_idle_at.peek() > 3 {
                            nav.push(Route::Conversations {});
                        }
                    }
                }
            },
            onkeydown: {
                let route = route.clone();
                let nav = nav.clone();
                let state = state.clone();
                move |_| {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let mut last_activity = state.peek().last_activity;
                    last_activity.set(now);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now - *entered_idle_at.peek() > 3 {
                            nav.push(Route::Conversations {});
                        }
                    }
                }
            },
            onmousemove: {
                let route = route.clone();
                let nav = nav.clone();
                let state = state.clone();
                move |_| {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let mut last_activity = state.peek().last_activity;
                    last_activity.set(now);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now - *entered_idle_at.peek() > 3 {
                            nav.push(Route::Conversations {});
                        }
                    }
                }
            },

            // Hidden Debug Timer (visible only in dev)
            div { class: "fixed bottom-0 right-0 p-1 text-[8px] opacity-10 pointer-events-none z-[9999]",
                "IDLE: {idle_time}s | GRACE: {std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() - *entered_idle_at.peek()}s"
            }
            
            Outlet::<Route> {}
        }
    }
}

