use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use std::time::Duration;

/// Shows pending Kernel approvals (from `kernel_execute` require-approval) with
/// Approve / Reject buttons wired to `engine::kernel::approve` / `reject`. It
/// polls so a governed action that suspends waiting for a human shows up here.
#[component]
pub fn KernelApprovalBar() -> Element {
    let db = crate::state::use_global_state().read().db.clone();

    // Poll ~every 1.2s.
    let mut tick = use_signal(|| 0u32);
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            tick += 1;
        }
    });

    let db_for_res = db.clone();
    let pending = use_resource(move || {
        let db = db_for_res.clone();
        let _ = tick();
        async move { crate::engine::kernel::list_pending_approvals(&db, None).await }
    });

    let items = pending.read().clone().unwrap_or_default();
    if items.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "px-3 sm:px-6 pt-3",
            div { class: "w-full max-w-3xl mx-auto space-y-2",
                for a in items {
                    div { class: "rounded-2xl border border-amber-300 bg-amber-50 px-4 py-3",
                        div { class: "flex items-start gap-2",
                            MaterialIcon { name: "shield", size: 18 }
                            div { class: "min-w-0 flex-1",
                                p { class: "text-xs font-semibold text-amber-800 uppercase tracking-wide", "Approval required" }
                                p { class: "text-sm text-amber-900 mt-0.5 break-words",
                                    "{a.summary.clone().unwrap_or_else(|| a.tool_name.clone())}"
                                }
                                if let Some(reason) = a.reason.clone() {
                                    p { class: "text-xs text-amber-700 mt-0.5", "{reason}" }
                                }
                            }
                        }
                        div { class: "flex items-center gap-2 mt-2",
                            button {
                                class: "bg-emerald-600 text-white px-3 py-1.5 rounded-lg text-xs font-semibold hover:bg-emerald-700",
                                onclick: {
                                    let db = db.clone();
                                    let id = a.id.clone();
                                    move |_| {
                                        crate::engine::kernel::approve(&db, &id, "user");
                                        tick += 1;
                                    }
                                },
                                "Approve"
                            }
                            button {
                                class: "border border-red-200 bg-white text-red-600 px-3 py-1.5 rounded-lg text-xs font-semibold hover:bg-red-50",
                                onclick: {
                                    let db = db.clone();
                                    let id = a.id.clone();
                                    move |_| {
                                        crate::engine::kernel::reject(&db, &id, "user");
                                        tick += 1;
                                    }
                                },
                                "Reject"
                            }
                        }
                    }
                }
            }
        }
    }
}
