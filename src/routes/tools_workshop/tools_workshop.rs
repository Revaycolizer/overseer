use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn ToolsWorkshop() -> Element {
    let mut global_state = crate::state::use_global_state();
    let db = global_state.read().db.clone();

    let db_resource = db.clone();
    let mut tools_resource =
        use_resource::<Vec<crate::engine::utils::discovery::ToolInfo>, _>(move || {
            let db_inner = db_resource.clone();
            async move { backend::get_tools(&db_inner).await.unwrap_or_default() }
        });

    let tools = tools_resource.read().cloned().unwrap_or_default();
    let mut selected_tool_id = use_signal(|| tools.first().map(|t| t.id).unwrap_or(0i64));
    let mut selected_version_tag = use_signal(String::new);

    let selected_tool = tools.iter().find(|t| t.id == selected_tool_id()).cloned();
    let mut local_code = use_signal(|| {
        selected_tool
            .as_ref()
            .map(|t| t.code.clone())
            .unwrap_or_default()
    });

    let mut is_fullscreen = use_signal(|| false);
    let mut test_output = use_signal(|| "Run a test to see output here.".to_string());
    let mut is_testing = use_signal(|| false);
    let mut test_query = use_signal(|| String::new());
    let mut workbench_status = use_signal(|| "Tool workbench ready.".to_string());

    // Sync code and handle initial selection
    let tools_for_effect = tools.clone();
    use_effect(move || {
        let current_tools = tools_for_effect.clone();
        if selected_tool_id() == 0 {
            if let Some(first) = current_tools.first() {
                selected_tool_id.set(first.id);
            }
        }

        if let Some(tool) = current_tools.iter().find(|t| t.id == selected_tool_id()) {
            local_code.set(tool.code.clone());
        }
        selected_version_tag.set(String::new());
    });

    let db_for_versions = db.clone();
    let mut version_history_resource = use_resource(move || {
        let db = db_for_versions.clone();
        let tool_id = selected_tool_id();
        async move {
            if tool_id != 0 {
                backend::get_version_history(&db, "tool", tool_id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let db_for_version_content = db.clone();
    let version_content_resource = use_resource(move || {
        let db = db_for_version_content.clone();
        let tool_id = selected_tool_id();
        let version_tag = selected_version_tag();
        async move {
            if tool_id != 0 && !version_tag.is_empty() {
                backend::get_version_content(&db, "tool", tool_id, &version_tag)
                    .await
                    .unwrap_or_default()
            } else {
                None
            }
        }
    });

    // Re-highlight when code changes
    let _code_trigger = local_code();
    use_effect(move || {
        let _ = _code_trigger;
        let script = "setTimeout(() => { if (window.Prism) { Prism.highlightAll(); } }, 10);";
        document::eval(script);
    });

    let code_dirty = selected_tool
        .as_ref()
        .map(|tool| tool.code != local_code())
        .unwrap_or(false);
    let intent_summary = selected_tool
        .as_ref()
        .and_then(|tool| tool.required_intent.clone())
        .unwrap_or_else(|| "No intent guardrails configured.".to_string());

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-background-light overflow-y-auto custom-scrollbar p-8",
            div { class: "max-w-7xl mx-auto w-full",
                div { class: "mb-10 flex flex-col md:flex-row md:items-end justify-between gap-6",
                    div {
                        h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                            "Tools Workshop"
                        }
                        p { class: "text-base text-slate-500 max-w-2xl",
                            " Create, test, and deploy functional tools for your AI to interact with the OS. "
                        }
                    }
                    div { class: "flex items-center gap-3",
                        button {
                            class: "bg-primary text-white px-4 py-2 rounded-lg text-sm font-semibold flex items-center gap-2 hover:bg-slate-800 shadow-sm transition-all active:scale-[0.98]",
                            onclick: {
                                let db_click = db.clone();
                                let mut tools_resource_click = tools_resource.clone();
                                move |_| {
                                    let db_spawn = db_click.clone();
                                    let mut tools_resource_spawn = tools_resource_click.clone();
                                    spawn(async move {
                                        let _ = backend::add_tool(&db_spawn, "New Tool", "A new functional tool", "python").await;
                                        tools_resource_spawn.restart();
                                    });
                                }
                            },
                            MaterialIcon { name: "add", size: 18, color: MaterialIconColor::Light }
                            " New Tool "
                        }
                    }
                }
                div { class: "grid grid-cols-1 lg:grid-cols-3 gap-8",
                    div { class: "lg:col-span-1 space-y-4",
                        div { class: "flex items-center justify-between mb-2",
                            h3 { class: "text-xs font-bold text-slate-400 uppercase tracking-widest",
                                "Local Library"
                            }
                            span { class: "text-[10px] bg-slate-200 text-slate-600 px-2 py-0.5 rounded-full font-bold",
                                "{tools.len()} Total"
                            }
                        }
                        for tool in tools.clone().into_iter() {
                            div {
                                class: format!("bg-white border-2 rounded-xl p-4 shadow-sm cursor-pointer transition-all {}", if selected_tool_id() == tool.id { "border-brand-blue shadow-md" } else { "border-slate-100 hover:border-slate-200" }),
                                onclick: {
                                    let tool_id = tool.id;
                                    move |_| selected_tool_id.set(tool_id)
                                },
                                div { class: "flex items-center justify-between mb-3",
                                    div { class: "flex items-center gap-3",
                                        div { class: "w-8 h-8 bg-blue-50 text-brand-blue rounded flex items-center justify-center",
                                            MaterialIcon {
                                                name: "build",
                                                size: 20,
                                                color: MaterialIconColor::Custom("text-brand-blue".into())
                                            }
                                        }
                                        span { class: "font-bold text-slate-900 text-sm",
                                            "{tool.name}"
                                        }
                                    }
                                    span { class: "w-2 h-2 rounded-full bg-emerald-500" }
                                }
                                p { class: "text-xs text-slate-500 line-clamp-1 mb-3",
                                    "{tool.description}"
                                }
                                div { class: "flex items-center gap-2",
                                    span { class: "text-[10px] font-mono bg-slate-100 text-slate-500 px-1.5 py-0.5 rounded",
                                        "{tool.language}"
                                    }
                                    span { class: "text-[10px] text-slate-400", "Local System" }
                                }
                            }
                        }
                    }
                    div { class: "lg:col-span-2 space-y-6",
                        if let Some(tool) = selected_tool {
                            div { class: "bg-white border border-slate-200 rounded-xl shadow-sm flex flex-col h-[600px] max-h-[600px]",
                                div { class: "border-b border-slate-200 p-4 flex items-center justify-between",
                                    div { class: "flex items-center gap-4",
                                        div { class: "space-y-3",
                                            div { class: "flex items-center gap-2 flex-wrap",
                                                span { class: "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full bg-slate-100 text-slate-600",
                                                    "{tool.language}"
                                                }
                                                span { class: format!(
                                                    "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                                    if code_dirty {
                                                        "bg-amber-50 text-amber-700"
                                                    } else {
                                                        "bg-emerald-50 text-emerald-600"
                                                    }
                                                ),
                                                    if code_dirty { "Draft Logic" } else { "Synced" }
                                                }
                                                span { class: format!(
                                                    "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                                    match tool.safety_level.as_str() {
                                                        "dangerous" => "bg-red-50 text-red-600",
                                                        "high" => "bg-amber-50 text-amber-700",
                                                        _ => "bg-blue-50 text-blue-600",
                                                    }
                                                ),
                                                    "{tool.safety_level}"
                                                }
                                            }
                                            div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-1", "Tool Name" }
                                                input {
                                                    class: "font-bold text-slate-900 border-none focus:ring-0 p-0",
                                                    value: "{tool.name}",
                                                    oninput: {
                                                        let mut tools_resource = tools_resource.clone();
                                                        let db = db.clone();
                                                        let tool_for_name = tool.clone();
                                                        move |e| {
                                                            let new_name = e.value();
                                                            let db = db.clone();
                                                            let mut tools_resource = tools_resource.clone();
                                                            let mut updated_tool = tool_for_name.clone();
                                                            updated_tool.name = new_name;
                                                            spawn(async move {
                                                                let _ = backend::update_tool(&db, updated_tool).await;
                                                                tools_resource.restart();
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-2 shrink-0",
                                        button { class: "text-slate-400 hover:text-slate-600 p-1.5 rounded-md hover:bg-slate-50",
                                            onclick: move |_| is_fullscreen.set(!is_fullscreen()),
                                            MaterialIcon { name: if is_fullscreen() { "fullscreen_exit" } else { "fullscreen" }, size: 20 }
                                        }
                                        button {
                                            class: "bg-blue-600 text-white px-3 py-1.5 rounded-lg text-[10px] font-bold flex items-center gap-2 hover:bg-blue-700 transition-all shadow-sm active:scale-95",
                                            onclick: {
                                                let t = tool.clone();
                                                let local_code = local_code.clone();
                                                let query = test_query.clone();
                                                move |_| {
                                                    let lang = t.language.clone();
                                                    let code = local_code();
                                                    let q = query();
                                                    is_testing.set(true);
                                                    test_output.set("Running test...".to_string());
                                                    workbench_status.set("Executing tool test...".to_string());
                                                    spawn(async move {
                                                        let query_opt = if q.is_empty() { None } else { Some(q.as_str()) };
                                                        match backend::test_tool_code(&lang, &code, query_opt).await {
                                                            Ok(res) => {
                                                                test_output.set(res);
                                                                workbench_status.set("Tool test completed.".to_string());
                                                            }
                                                            Err(e) => {
                                                                test_output.set(format!("Test Error: {}", e));
                                                                workbench_status.set("Tool test failed.".to_string());
                                                            }
                                                        }
                                                        is_testing.set(false);
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "play_arrow", size: 14, color: MaterialIconColor::Light }
                                            if is_testing() { " Testing... " } else { " Test Tool " }
                                        }
                                        button {
                                            class: "bg-emerald-600 text-white px-3 py-1.5 rounded-lg text-[10px] font-bold flex items-center gap-2 hover:bg-emerald-700 transition-all shadow-sm active:scale-95",
                                            onclick: {
                                                let t = tool.clone();
                                                let local_code = local_code.clone();
                                                let db_click = db.clone();
                                                let mut tools_resource_click = tools_resource.clone();
                                                move |_| {
                                                    let mut updated_tool = t.clone();
                                                    updated_tool.code = local_code();
                                                    let db_spawn = db_click.clone();
                                                    let mut tools_resource_spawn = tools_resource_click.clone();
                                                    let tool_id = t.id;
                                                    let mut version_history_resource = version_history_resource.clone();
                                                    workbench_status.set("Publishing snapshot...".to_string());
                                                    spawn(async move {
                                                        let version_tag = format!("snapshot-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
                                                        let _ = backend::update_tool(&db_spawn, updated_tool).await;
                                                        let _ = backend::publish_tool_version(&db_spawn, tool_id, &version_tag, "Version snapshot").await;
                                                        tools_resource_spawn.restart();
                                                        version_history_resource.restart();
                                                        workbench_status.set("Snapshot published and logic synced.".to_string());
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "publish", size: 14 }
                                            " Publish Version "
                                        }
                                        button {
                                            class: "bg-slate-900 text-white px-3 py-1.5 rounded-lg text-[10px] font-bold flex items-center gap-2 hover:bg-slate-800 transition-all shadow-sm active:scale-95",
                                            onclick: {
                                                let t = tool.clone();
                                                let local_code = local_code.clone();
                                                let db_click = db.clone();
                                                let mut tools_resource_click = tools_resource.clone();
                                                move |_| {
                                                    let mut updated_tool = t.clone();
                                                    updated_tool.code = local_code();
                                                    let db_spawn = db_click.clone();
                                                    let mut tools_resource_spawn = tools_resource_click.clone();
                                                    workbench_status.set("Saving tool logic...".to_string());
                                                    spawn(async move {
                                                        let _ = backend::update_tool(&db_spawn, updated_tool).await;
                                                        tools_resource_spawn.restart();
                                                        workbench_status.set("Tool logic saved.".to_string());
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "save", size: 14 }
                                            " Save & Sync Logic "
                                        }
                                        div { class: "w-[1px] h-4 bg-slate-200 mx-1" }
                                        button { class: "text-slate-400 hover:text-slate-600 p-1.5 rounded-md hover:bg-slate-50",
                                            MaterialIcon { name: "settings", size: 20 }
                                        }
                                    }
                                }
                                div { class: "border-b border-slate-100 px-4 py-3 bg-slate-50/70 flex flex-col gap-2 xl:flex-row xl:items-center xl:justify-between",
                                    p { class: "text-xs text-slate-600", "{workbench_status}" }
                                    p { class: "text-[11px] text-slate-500",
                                        "Intent guardrail: {intent_summary}"
                                    }
                                }
                                div {
                                    class: if is_fullscreen() {
                                        "fixed inset-0 z-50 bg-[#1e1e1e] flex flex-col p-6 animate-fade-in"
                                    } else {
                                        "flex-1 overflow-hidden relative code-editor-container min-h-[300px]"
                                    },
                                    if is_fullscreen() {
                                        div { class: "flex justify-between items-center mb-6",
                                            div { class: "flex items-center gap-3 text-white",
                                                MaterialIcon { name: "code", size: 24, color: MaterialIconColor::Light }
                                                span { class: "font-bold font-mono text-sm", "{tool.name}.{tool.language}" }
                                            }
                                            div { class: "flex items-center gap-4",
                                                button {
                                                    class: "bg-blue-600 text-white px-5 py-2.5 rounded-lg text-xs font-bold flex items-center gap-2 hover:bg-blue-500 shadow-md",
                                                    onclick: {
                                                        let t = tool.clone();
                                                        let local_code = local_code.clone();
                                                        let query = test_query.clone();
                                                        move |_| {
                                                            let lang = t.language.clone();
                                                            let code = local_code();
                                                            let q = query();
                                                            is_testing.set(true);
                                                            test_output.set("Running test...".to_string());
                                                            workbench_status.set("Executing tool test...".to_string());
                                                            spawn(async move {
                                                                let query_opt = if q.is_empty() { None } else { Some(q.as_str()) };
                                                                match backend::test_tool_code(&lang, &code, query_opt).await {
                                                                    Ok(res) => {
                                                                        test_output.set(res);
                                                                        workbench_status.set("Tool test completed.".to_string());
                                                                    }
                                                                    Err(e) => {
                                                                        test_output.set(format!("Test Error: {}", e));
                                                                        workbench_status.set("Tool test failed.".to_string());
                                                                    }
                                                                }
                                                                is_testing.set(false);
                                                            });
                                                        }
                                                    },
                                                    MaterialIcon { name: "play_arrow", size: 16, color: MaterialIconColor::Light }
                                                    if is_testing() { " Testing... " } else { " Run Test " }
                                                }
                                                button { class: "bg-white/10 hover:bg-white/20 text-white p-2.5 rounded-lg flex items-center transition-colors",
                                                    onclick: move |_| is_fullscreen.set(false),
                                                    MaterialIcon { name: "fullscreen_exit", size: 20, color: MaterialIconColor::Light }
                                                }
                                            }
                                        }
                                    }

                                    div { class: if is_fullscreen() { "flex-1 relative overflow-hidden bg-black/20 rounded-xl" } else { "w-full h-full relative" },
                                        textarea {
                                            class: "code-editor-textarea",
                                            value: "{local_code}",
                                            oninput: move |e| local_code.set(e.value()),
                                            spellcheck: false,
                                        }
                                        pre { class: "code-editor-highlight language-{tool.language}",
                                            code { "{local_code}\n\n" }
                                        }
                                    }
                                }

                                // Tool Metadata Section
                                div { class: "border-t border-slate-200 p-6 bg-white",
                                    div { class: "grid grid-cols-2 gap-6",
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Description" }
                                            textarea {
                                                class: "w-full bg-slate-50 border border-slate-100 rounded-lg p-2 text-xs text-slate-600 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue transition-all min-h-[60px]",
                                                value: "{tool.description}",
                                                oninput: {
                                                    let mut tools_resource = tools_resource.clone();
                                                    let db = db.clone();
                                                    let tool_for_desc = tool.clone();
                                                    move |e| {
                                                        let new_desc = e.value();
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut updated_tool = tool_for_desc.clone();
                                                        updated_tool.description = new_desc;
                                                        spawn(async move {
                                                            let _ = backend::update_tool(&db, updated_tool).await;
                                                            tools_resource.restart();
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Safety Level" }
                                            select {
                                                class: "w-full bg-slate-50 border border-slate-100 rounded-lg p-2 text-xs font-semibold text-slate-600 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue transition-all",
                                                value: "{tool.safety_level}",
                                                onchange: {
                                                    let mut tools_resource = tools_resource.clone();
                                                    let db = db.clone();
                                                    let tool_for_safety = tool.clone();
                                                    move |e| {
                                                        let new_level = e.value();
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut updated_tool = tool_for_safety.clone();
                                                        updated_tool.safety_level = new_level;
                                                        spawn(async move {
                                                            let _ = backend::update_tool(&db, updated_tool).await;
                                                            tools_resource.restart();
                                                        });
                                                    }
                                                },
                                                option { value: "standard", "Standard" }
                                                option { value: "high", "High Risk" }
                                                option { value: "dangerous", "Dangerous" }
                                            }
                                        }
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Legacy Script Path (Fallback)" }
                                            input {
                                                class: "w-full bg-slate-50 border border-slate-100 rounded-lg p-2 text-xs font-mono text-slate-600",
                                                placeholder: "Leave empty to use DB code exclusively",
                                                value: tool.script_path.clone().unwrap_or_default(),
                                                oninput: {
                                                    let mut tools_resource = tools_resource.clone();
                                                    let db = db.clone();
                                                    let tool_for_path = tool.clone();
                                                    move |e| {
                                                        let new_path = e.value();
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut updated_tool = tool_for_path.clone();
                                                        updated_tool.script_path = if new_path.is_empty() { None } else { Some(new_path) };
                                                        spawn(async move {
                                                            let _ = backend::update_tool(&db, updated_tool).await;
                                                            tools_resource.restart();
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "mt-4",
                                        label { class: "block text-[10px] font-bold text-slate-1000 uppercase tracking-widest mb-2",
                                            span { class: "text-brand-blue", "🛡️ Intent Guardrail Keywords" }
                                        }
                                        input {
                                            class: "w-full bg-slate-50 border border-slate-100 rounded-lg p-2 text-xs font-mono text-slate-600 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue transition-all",
                                            placeholder: "e.g., chart,graph,plot (comma separated). Tool will only run if query contains these.",
                                            value: tool.required_intent.clone().unwrap_or_default(),
                                            oninput: {
                                                let mut tools_resource = tools_resource.clone();
                                                let db = db.clone();
                                                let tool_for_intent = tool.clone();
                                                move |e| {
                                                    let new_intent = e.value();
                                                    let db = db.clone();
                                                    let mut tools_resource = tools_resource.clone();
                                                    let mut updated_tool = tool_for_intent.clone();
                                                    updated_tool.required_intent = if new_intent.is_empty() { None } else { Some(new_intent) };
                                                    spawn(async move {
                                                        let _ = backend::update_tool(&db, updated_tool).await;
                                                        tools_resource.restart();
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }

                                // Agent relationships section
                                div { class: "border-t border-slate-200 p-4 bg-slate-50",
                                    h4 { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3", "Used by Agents" }
                                    div { class: "flex flex-wrap gap-2",
                                        {
                                            let agents = backend::get_agents(&db);
                                            rsx! {
                                                span { class: "text-[10px] text-slate-400 italic", "Agent relationship surfacing is still lightweight here. Use this area as the assignment checkpoint before publishing." }
                                            }
                                        }
                                    }
                                }

                                div { class: "border-t border-slate-200 p-4 bg-white",
                                    div { class: "flex items-center justify-between mb-4",
                                        h4 { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Version History" }
                                        if !selected_version_tag().is_empty() {
                                            button {
                                                class: "bg-slate-900 text-white px-3 py-1.5 rounded-lg text-[10px] font-bold hover:bg-slate-800 transition-all",
                                                onclick: {
                                                    let db = db.clone();
                                                    let mut tools_resource = tools_resource.clone();
                                                    let mut version_history_resource = version_history_resource.clone();
                                                    let version_tag = selected_version_tag();
                                                    move |_| {
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut version_history_resource = version_history_resource.clone();
                                                        let version_tag = version_tag.clone();
                                                        spawn(async move {
                                                            let _ = backend::rollback_version(&db, "tool", tool.id, &version_tag).await;
                                                            tools_resource.restart();
                                                            version_history_resource.restart();
                                                        });
                                                    }
                                                },
                                                "Rollback to Snapshot"
                                            }
                                        }
                                    }
                                    div { class: "grid grid-cols-1 xl:grid-cols-[240px_minmax(0,1fr)] gap-4",
                                        div { class: "space-y-2 max-h-48 overflow-y-auto custom-scrollbar pr-1",
                                            for version in version_history_resource.read().cloned().unwrap_or_default() {
                                                button {
                                                    key: "{version.id}",
                                                    class: format!(
                                                        "w-full text-left rounded-xl border px-3 py-2 transition-all {}",
                                                        if selected_version_tag() == version.version_tag {
                                                            "border-blue-300 bg-blue-50"
                                                        } else {
                                                            "border-slate-200 bg-slate-50 hover:border-slate-300"
                                                        }
                                                    ),
                                                    onclick: {
                                                        let tag = version.version_tag.clone();
                                                        move |_| selected_version_tag.set(tag.clone())
                                                    },
                                                    div { class: "text-[11px] font-bold text-slate-700", "{version.version_tag}" }
                                                    div { class: "text-[10px] text-slate-400 mt-1", {version.created_at.format("%Y-%m-%d %H:%M").to_string()} }
                                                    div { class: "text-[10px] text-slate-500 mt-1 line-clamp-2", "{version.changelog.clone().unwrap_or_default()}" }
                                                }
                                            }
                                        }
                                        div { class: "grid grid-cols-1 xl:grid-cols-2 gap-4",
                                            div {
                                                label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Current Logic" }
                                                pre { class: "min-h-[180px] max-h-[240px] overflow-auto custom-scrollbar bg-slate-50 border border-slate-200 rounded-xl p-3 text-[11px] text-slate-700 whitespace-pre-wrap",
                                                    "{local_code}"
                                                }
                                            }
                                            div {
                                                label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Selected Snapshot" }
                                                pre { class: "min-h-[180px] max-h-[240px] overflow-auto custom-scrollbar bg-slate-50 border border-slate-200 rounded-xl p-3 text-[11px] text-slate-700 whitespace-pre-wrap",
                                                    "{version_content_resource.read().cloned().flatten().unwrap_or_else(|| \"Select a snapshot to compare or rollback.\".to_string())}"
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "border-t border-slate-200 p-4 bg-white rounded-b-xl space-y-3",
                                    div { class: "flex items-center justify-between mb-2",
                                        div {
                                            span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Test Harness" }
                                            p { class: "text-[11px] text-slate-500 mt-1", "Run the current draft against a sample query before saving or publishing." }
                                        }
                                        span { class: format!(
                                            "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                            if is_testing() {
                                                "bg-blue-50 text-blue-600"
                                            } else if code_dirty {
                                                "bg-amber-50 text-amber-700"
                                            } else {
                                                "bg-emerald-50 text-emerald-600"
                                            }
                                        ),
                                            if is_testing() { "Running" } else if code_dirty { "Draft" } else { "Ready" }
                                        }
                                    }
                                    div { class: "mb-3 flex items-center gap-2",
                                        input {
                                            class: "flex-1 bg-slate-50 border border-slate-200 rounded-lg p-2 text-xs font-mono text-slate-600 focus:ring-1 focus:ring-brand-blue/20",
                                            placeholder: "Type a sample query or tool input to simulate a real run",
                                            value: "{test_query}",
                                            oninput: move |e| test_query.set(e.value()),
                                        }
                                    }
                                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-3 text-[11px]",
                                        div { class: "rounded-lg border border-slate-200 bg-slate-50 px-3 py-2",
                                            span { class: "block text-[10px] font-bold uppercase tracking-widest text-slate-400 mb-1", "Language" }
                                            "{tool.language}"
                                        }
                                        div { class: "rounded-lg border border-slate-200 bg-slate-50 px-3 py-2",
                                            span { class: "block text-[10px] font-bold uppercase tracking-widest text-slate-400 mb-1", "Safety" }
                                            "{tool.safety_level}"
                                        }
                                        div { class: "rounded-lg border border-slate-200 bg-slate-50 px-3 py-2",
                                            span { class: "block text-[10px] font-bold uppercase tracking-widest text-slate-400 mb-1", "Guardrail" }
                                            "{intent_summary}"
                                        }
                                    }
                                    pre { class: "text-[10px] text-slate-600 font-mono whitespace-pre-wrap bg-slate-50 p-3 rounded-lg border border-slate-100 max-h-40 overflow-y-auto custom-scrollbar",
                                        "{test_output}"
                                    }
                                }
                            }
                        } else {
                            div { class: "flex-1 flex items-center justify-center bg-white border border-slate-200 rounded-xl",
                                p { class: "text-slate-400", "Select a tool to view details" }
                            }
                        }
                    }
                }
                footer { class: "py-12 border-t border-slate-200 mt-12 px-8",
                    div { class: "max-w-7xl mx-auto flex flex-col md:flex-row justify-between items-center gap-8",
                        div { class: "flex items-center gap-2 text-slate-400",
                            MaterialIcon { name: "verified_user", size: 18 }
                            span { class: "text-[11px] font-bold uppercase tracking-widest",
                                "Secure Local Environment • v2.4.0"
                            }
                        }
                        div { class: "flex gap-10",
                            a {
                                class: "text-[11px] font-bold text-slate-400 hover:text-slate-900 uppercase tracking-widest transition-colors",
                                href: "#",
                                "Documentation"
                            }
                            a {
                                class: "text-[11px] font-bold text-slate-400 hover:text-slate-900 uppercase tracking-widest transition-colors",
                                href: "#",
                                "Privacy Policy"
                            }
                        }
                    }
                }
            }
        }
    }
}
