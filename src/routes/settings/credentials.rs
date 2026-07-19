use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

/// A managed credential displayed in the settings panel.
#[derive(Clone, PartialEq)]
struct Credential {
    id: i64,
    name: String,
    cred_type: String,
}

/// Settings section for managing API keys and credentials.
///
/// Credentials are stored encrypted in the database and used by
/// workflow steps and tool executions that require external service auth.
/// The encrypted values are never displayed — only the name and type.
#[component]
pub fn CredentialsSection() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();

    // Reload trigger
    let mut reload = use_signal(|| 0u32);

    let credentials = use_resource({
        let db = db.clone();
        move || {
            let _ = *reload.read(); // subscribe to reloads
            load_credentials(db.clone())
        }
    });

    let mut show_add_form = use_signal(|| false);
    let mut new_name = use_signal(|| String::new());
    let mut new_value = use_signal(|| String::new());
    let mut new_type = use_signal(|| "api_key".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut status_msg = use_signal(|| String::new());

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white p-6 space-y-5",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    MaterialIcon { name: "key", size: 22 }
                    h3 { class: "text-lg font-bold text-slate-900", "Credentials" }
                }
                button {
                    class: "inline-flex items-center gap-1.5 rounded-lg bg-slate-900 px-3 py-1.5 text-xs font-semibold text-white hover:bg-slate-700 transition-all",
                    onclick: move |_| show_add_form.set(true),
                    MaterialIcon { name: "add", size: 14 }
                    "Add Credential"
                }
            }

            p { class: "text-sm text-slate-500 -mt-2",
                "Encrypted API keys and secrets used by tools and workflow steps."
            }

            // Status / Error display
            if !error_msg.read().is_empty() {
                div { class: "rounded-lg bg-red-50 border border-red-200 px-4 py-3 text-sm text-red-700",
                    "{error_msg}"
                }
            }
            if !status_msg.read().is_empty() {
                div { class: "rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-700",
                    "{status_msg}"
                }
            }

            // Add form
            if *show_add_form.read() {
                div { class: "rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-3",
                    h4 { class: "text-sm font-semibold text-slate-700", "New Credential" }
                    
                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                        input {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                            placeholder: "Name (e.g. OPENAI_API_KEY)",
                            value: "{new_name}",
                            oninput: move |e| new_name.set(e.value()),
                        }
                        select {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                            value: "{new_type}",
                            onchange: move |e| new_type.set(e.value()),
                            option { value: "api_key", "API Key" }
                            option { value: "password", "Password" }
                            option { value: "token", "Token" }
                            option { value: "custom", "Custom" }
                        }
                        input {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                            placeholder: "Value (will be encrypted)",
                            value: "{new_value}",
                            oninput: move |e| new_value.set(e.value()),
                            r#type: "password",
                        }
                    }

                    div { class: "flex justify-end gap-2",
                        button {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs font-medium text-slate-600 hover:bg-slate-50 transition-all",
                            onclick: move |_| {
                                show_add_form.set(false);
                                new_name.set(String::new());
                                new_value.set(String::new());
                                error_msg.set(String::new());
                                status_msg.set(String::new());
                            },
                            "Cancel"
                        }
                        button {
                            class: "rounded-lg bg-slate-900 px-3 py-1.5 text-xs font-semibold text-white hover:bg-slate-700 transition-all",
                            onclick: move |_| {
                                let name = new_name.read().clone();
                                let value = new_value.read().clone();
                                let ct = new_type.read().clone();
                                if name.trim().is_empty() || value.trim().is_empty() {
                                    error_msg.set("Name and value are required.".to_string());
                                    return;
                                }
                                let db = db.clone();
                                let name_c = name.clone();
                                let ct_c = ct.clone();
                                let value_c = value.clone();
                                spawn(async move {
                                    match crate::engine::workflow::credentials::encrypt(&value_c) {
                                        Ok(encrypted) => {
                                            let id = db.next_id().await;
                                            let result = sqlx::query(
                                                "INSERT INTO credentials (id, name, cred_type, encrypted_value, owner_id) VALUES (?, ?, ?, ?, 1)"
                                            )
                                            .bind(id)
                                            .bind(&name_c)
                                            .bind(&ct_c)
                                            .bind(&encrypted)
                                            .execute(&db.pool)
                                            .await;
                                            match result {
                                                Ok(_) => {
                                                    new_name.set(String::new());
                                                    new_value.set(String::new());
                                                    show_add_form.set(false);
                                                    error_msg.set(String::new());
                                                    status_msg.set(format!("Credential '{}' saved.", name_c));
                                                    reload += 1;
                                                }
                                                Err(e) => error_msg.set(e.to_string()),
                                            }
                                        }
                                        Err(e) => error_msg.set(e.to_string()),
                                    }
                                });
                            },
                            "Save"
                        }
                    }
                }
            }

            // Credential list
            match credentials.read().as_ref() {
                Some(Ok(list)) if list.is_empty() => {
                    div { class: "rounded-xl border-2 border-dashed border-slate-200 p-6 text-center",
                        p { class: "text-xs font-medium text-slate-400",
                            "No credentials stored. Add an API key or secret above."
                        }
                    }
                }
                Some(Ok(list)) => {
                    div { class: "space-y-2",
                        for cred in list {
                            div { class: "flex items-center justify-between rounded-xl border border-slate-100 bg-slate-50/50 px-4 py-3",
                                div { class: "flex items-center gap-3",
                                    MaterialIcon { 
                                        name: match cred.cred_type.as_str() {
                                            "api_key" => "vpn_key",
                                            "password" => "lock",
                                            "token" => "confirmation_number",
                                            _ => "key",
                                        }, 
                                        size: 16 
                                    }
                                    div {
                                        p { class: "text-sm font-medium text-slate-900", "{cred.name}" }
                                        p { class: "text-xs text-slate-400", "{cred.cred_type}" }
                                    }
                                }
                                button {
                                    class: "rounded-lg px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-50 transition-all",
                                    onclick: move |_| {
                                        let db = db.clone();
                                        let id = cred.id;
                                        spawn(async move {
                                            let _ = sqlx::query("DELETE FROM credentials WHERE id = ?")
                                                .bind(id)
                                                .execute(&db.pool)
                                                .await;
                                            reload += 1;
                                        });
                                    },
                                    "Delete"
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    div { class: "rounded-lg bg-red-50 p-4 text-sm text-red-700",
                        "Failed to load credentials: {e}"
                    }
                }
                None => {
                    div { class: "text-center py-4",
                        p { class: "text-sm text-slate-400", "Loading credentials..." }
                    }
                }
            }
        }
    }
}

async fn load_credentials(db: std::sync::Arc<crate::db::Database>) -> Result<Vec<Credential>, String> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, cred_type FROM credentials ORDER BY created_at DESC"
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(rows.into_iter().map(|(id, name, cred_type)| Credential { id, name, cred_type }).collect())
}
