use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use std::sync::Arc;
use crate::db::Database;

#[derive(Clone, PartialEq)]
struct Credential {
    id: i64,
    name: String,
    cred_type: String,
}

#[component]
fn CredentialRow(cred: Credential, db: Arc<Database>, reload: Signal<u32>) -> Element {
    let on_delete = move |_| {
        let db = db.clone();
        let cid = cred.id;
        spawn(async move {
            let _ = sqlx::query("DELETE FROM credentials WHERE id = ?")
                .bind(cid)
                .execute(&db.pool)
                .await;
            reload += 1;
        });
    };

    rsx! {
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
                onclick: on_delete,
                "Delete"
            }
        }
    }
}

#[component]
pub fn CredentialsSection() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();
    let db_signal = use_signal(move || db);

    let credentials_data = use_resource({
        let db_s = db_signal.clone();
        move || load_credentials((*db_s.read()).clone())
    });

    let mut show_add = use_signal(|| false);
    let mut cred_name = use_signal(|| String::new());
    let mut cred_value = use_signal(|| String::new());
    let mut cred_type = use_signal(|| "api_key".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut status_msg = use_signal(|| String::new());
    let mut reload_flag = use_signal(|| 0u32);
    let _ = *reload_flag.read();

    let cred_list: Vec<Credential> = match credentials_data.read().clone() {
        Some(Ok(list)) => list,
        _ => vec![],
    };
    drop(credentials_data);

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white p-6 space-y-5",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    MaterialIcon { name: "key", size: 22 }
                    h3 { class: "text-lg font-bold text-slate-900", "Credentials" }
                }
                button {
                    class: "inline-flex items-center gap-1.5 rounded-lg bg-slate-900 px-3 py-1.5 text-xs font-semibold text-white hover:bg-slate-700 transition-all",
                    onclick: move |_| show_add.set(true),
                    MaterialIcon { name: "add", size: 14 }
                    "Add Credential"
                }
            }

            p { class: "text-sm text-slate-500 -mt-2",
                "Encrypted API keys and secrets used by tools and workflow steps."
            }

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

            if *show_add.read() {
                div { class: "rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-3",
                    h4 { class: "text-sm font-semibold text-slate-700", "New Credential" }
                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                        input {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                            placeholder: "Name (e.g. OPENAI_API_KEY)",
                            value: "{cred_name}",
                            oninput: move |e| cred_name.set(e.value()),
                        }
                        select {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                            value: "{cred_type}",
                            onchange: move |e| cred_type.set(e.value()),
                            option { value: "api_key", "API Key" }
                            option { value: "password", "Password" }
                            option { value: "token", "Token" }
                            option { value: "custom", "Custom" }
                        }
                        input {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                            placeholder: "Value (will be encrypted)",
                            value: "{cred_value}",
                            oninput: move |e| cred_value.set(e.value()),
                            r#type: "password",
                        }
                    }
                    div { class: "flex justify-end gap-2",
                        button {
                            class: "rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs font-medium text-slate-600 hover:bg-slate-50 transition-all",
                            onclick: move |_| {
                                show_add.set(false);
                                cred_name.set(String::new());
                                cred_value.set(String::new());
                                error_msg.set(String::new());
                                status_msg.set(String::new());
                            },
                            "Cancel"
                        }
                        button {
                            class: "rounded-lg bg-slate-900 px-3 py-1.5 text-xs font-semibold text-white hover:bg-slate-700 transition-all",
                            onclick: move |_| {
                                let name = cred_name.read().clone();
                                let value = cred_value.read().clone();
                                let ct = cred_type.read().clone();
                                let db = (*db_signal.read()).clone();
                                if name.trim().is_empty() || value.trim().is_empty() {
                                    error_msg.set("Name and value are required.".to_string());
                                    return;
                                }
                                let nc = name.clone();
                                spawn(async move {
                                    match crate::engine::workflow::credentials::encrypt(&value) {
                                        Ok(encrypted) => {
                                            let id = db.next_id().await;
                                            let r = sqlx::query(
                                                "INSERT INTO credentials (id, name, cred_type, encrypted_value, owner_id) VALUES (?, ?, ?, ?, 1)"
                                            ).bind(id).bind(&name).bind(&ct).bind(&encrypted).execute(&db.pool).await;
                                            match r {
                                                Ok(_) => {
                                                    cred_name.set(String::new());
                                                    cred_value.set(String::new());
                                                    show_add.set(false);
                                                    error_msg.set(String::new());
                                                    status_msg.set(format!("Credential '{}' saved.", nc));
                                                    reload_flag += 1;
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

            if cred_list.is_empty() {
                div { class: "rounded-xl border-2 border-dashed border-slate-200 p-6 text-center",
                    p { class: "text-xs font-medium text-slate-400",
                        "No credentials stored. Add an API key or secret above."
                    }
                }
            } else {
                div { class: "space-y-2",
                    for cred in cred_list {
                        CredentialRow {
                            cred: cred,
                            db: (*db_signal.read()).clone(),
                            reload: reload_flag,
                        }
                    }
                }
            }
        }
    }
}

async fn load_credentials(db: Arc<Database>) -> Result<Vec<Credential>, String> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, cred_type FROM credentials ORDER BY created_at DESC"
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id, name, ct)| Credential { id, name, cred_type: ct }).collect())
}
