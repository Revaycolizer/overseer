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
    
    // Check if we should run in CLI mode
    if args.len() > 1 && (args[1] == "info" || args[1] == "--info" || !args[1].starts_with("-")) {
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
    let db = use_resource(|| async move {
        Database::new().await.ok()
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        MaterialIconStylesheet {}
        
        if let Some(Some(database)) = db.cloned() {
            StateProvider {
                database: Arc::new(database),
                Router::<Route> {}
            }
        } else {
            div { class: "flex items-center justify-center h-screen bg-gray-900 text-white",
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
