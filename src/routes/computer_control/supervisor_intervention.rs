use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use crate::db::Database;
use std::sync::Arc;

#[component]
pub fn InterventionCard(
    db: Arc<Database>,
    pending_text: String,
    on_refresh: Callback<()>,
) -> Element {
    rsx! {
        div { class: "bg-blue-50/70 border border-blue-100 rounded-2xl p-4.5 space-y-4 shadow-sm",
            div { class: "flex items-center gap-2 text-blue-600",
                MaterialIcon { name: "error_outline", size: 16 }
                span { class: "text-[10px] font-black uppercase tracking-wider font-mono", "Active Intervention Required" }
            }
            p { class: "text-xs text-blue-900 leading-relaxed font-medium", "{pending_text}" }
            div { class: "flex flex-col gap-2.5",
                    button { class: "w-full bg-blue-600 hover:bg-blue-750 text-white py-3 rounded-xl font-bold text-xs shadow-md shadow-blue-500/10 transition-all transform active:scale-98 uppercase tracking-wider",
                    onclick: {
                        let db = db.clone();
                        let cb = on_refresh.clone();
                        move |_| { let db = db.clone(); let cb = cb.clone(); spawn(async move { let _ = crate::backend::approve_pending_computer_action(&db).await; cb.call(()); }); }
                    },
                    "Approve Modification" }
                button { class: "w-full bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 py-3 rounded-xl font-bold text-xs transition-all transform active:scale-98 uppercase tracking-wider shadow-sm",
                    onclick: {
                        let db = db.clone();
                        let cb = on_refresh.clone();
                        move |_| { let db = db.clone(); let cb = cb.clone(); spawn(async move { let _ = crate::backend::reject_pending_computer_action(&db).await; cb.call(()); }); }
                    },
                    "Detailed Review" }
            }
        }
    }
}
