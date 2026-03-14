use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIconStylesheet};
mod routes;
mod backend;
mod state;
pub mod engine;
pub mod db;

use routes::{Conversations, AgentsWorkshop, Layout, WorkflowAutomation, Settings, ToolsWorkshop};
use state::StateProvider;
use crate::db::Database;
use std::sync::Arc;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Layout)]
        #[route("/")]
        Conversations {},
        #[route("/agents")]
        AgentsWorkshop {},
        #[route("/workflows")]
        WorkflowAutomation {},
        #[route("/tools")]
        ToolsWorkshop {},
        #[route("/settings")]
        Settings {},
    #[end_layout]
    #[route("/blog/:id")]
    Blog { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css",AssetOptions::css().with_static_head(true));
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css",AssetOptions::css().with_static_head(true));

pub fn common_head_content() -> String {

	format!(r#" 
					<link rel="icon" href="{FAVICON}">
					<link rel="stylesheet" href="{MAIN_CSS}" blocking="render">
					<link rel="stylesheet" href="{TAILWIND_CSS}" blocking="render">
				"#)
}
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
        let config = Config::new()
            .with_custom_head(common_head_content())
            .with_window(
                WindowBuilder::new()
                    .with_title("Overseer")
                    .with_decorations(false)
            );
        dioxus::LaunchBuilder::desktop().with_cfg(config).launch(App);
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
        result.ok()
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        
        // PrismJS for Syntax Highlighting
        document::Link { rel: "stylesheet", href: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css" }
        document::Script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js" }
        document::Script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-python.min.js" }
        document::Script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-rust.min.js" }
        document::Script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-markdown.min.js" }

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
                Some(database) => rsx! {
                    StateProvider {
                        database: Arc::new(database),
                        Router::<Route> {}
                    }
                },
                None => rsx! {
                    div { class: "flex items-center justify-center h-screen bg-[#0f1116] text-white flex-col",
                        div { class: "p-8 bg-red-900/20 border border-red-500 rounded-lg max-w-2xl",
                            h2 { class: "text-xl font-bold text-red-500 mb-4", "Database Initialization Failed" }
                            p { class: "text-gray-300 mb-4", "The application encountered an error while setting up the database. This usually happens if migration files have been modified after being applied." }
                            pre { class: "bg-black/40 p-4 rounded text-xs overflow-auto max-h-64 text-red-300",
                                "Error: Migration checksum mismatch or lock issue.\nCheck terminal logs for details."
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
            div { class: "flex items-center justify-center h-screen bg-[#0f1116] text-white",
                div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                p { class: "ml-4", "Initializing Database..." }
            }
        }
    }
}

#[component]
pub fn Hero() -> Element {
    rsx! {
        div {
            id: "hero",
            img { src: HEADER_SVG, id: "header" }
            div { id: "links",
                a { href: "https://dioxuslabs.com/learn/0.7/", "📚 Learn Dioxus" }
                a { href: "https://dioxuslabs.com/awesome", "🚀 Awesome Dioxus" }
                a { href: "https://github.com/dioxus-community/", "📡 Community Libraries" }
                a { href: "https://github.com/DioxusLabs/sdk", "⚙️ Dioxus Development Kit" }
                a { href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus", "💫 VSCode Extension" }
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}

/// Home page
#[component]
fn Home() -> Element {
    rsx! {
        Hero {}

    }
}

/// Blog page
#[component]
pub fn Blog(id: i32) -> Element {
    rsx! {
        div {
            id: "blog",

            // Content
            h1 { "This is blog #{id}!" }
            p { "In blog #{id}, we show how the Dioxus router works and how URL parameters can be passed as props to our route components." }

            // Navigation links
            Link {
                to: Route::Blog { id: id - 1 },
                "Previous"
            }
            span { " <---> " }
            Link {
                to: Route::Blog { id: id + 1 },
                "Next"
            }
        }
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        div {
            id: "navbar",
            Link {
                to: Route::Conversations {},
                "Home"
            }
            Link {
                to: Route::Blog { id: 1 },
                "Blog"
            }
        }

        Outlet::<Route> {}
    }
}
