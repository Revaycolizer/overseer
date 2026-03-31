use dioxus::prelude::*;
use crate::backend;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::routes::conversations::attachment_meta::get_attachment_meta;
use base64::{Engine as _, engine::general_purpose};

#[component]
pub fn AttachmentPreview(att: backend::Attachment) -> Element {
    let file_path = att.file_path.clone();
    let content_type = att.content_type.clone();
    let filename = att.filename.clone();

    let asset_data = use_resource(move || {
        let fp = file_path.clone();
        let ct = content_type.clone();
        async move {
            if fp.starts_with("/assets/") { Some(fp) }
            else {
                match std::fs::read(&fp) {
                    Ok(bytes) if !bytes.is_empty() => Some(format!("data:{};base64,{}", ct, general_purpose::STANDARD.encode(bytes))),
                    _ => None,
                }
            }
        }
    });

    let meta = get_attachment_meta(&att.content_type, &att.filename);
    let is_image = att.content_type.starts_with("image/");

    match asset_data().flatten() {
        Some(src) => if is_image {
            rsx! {
                div { class: "space-y-2",
                    a { href: "{src}", target: "_blank", class: "block group w-fit",
                        img { src: "{src}", alt: "{filename}", class: "rounded-2xl border border-slate-200 shadow-sm max-w-full h-auto block bg-white" }
                    }
                    div { class: "text-[10px] font-semibold text-slate-500 px-1 break-all", "{filename}" }
                }
            }
        } else {
            rsx! {
                a { href: "{src}", download: "{filename}", class: "inline-flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-4 py-3 text-[12px] font-bold text-slate-700 shadow-sm hover:shadow-md hover:border-blue-200 hover:bg-blue-50/30 transition-all group",
                    div { class: "size-8 rounded-lg bg-slate-50 flex items-center justify-center group-hover:bg-white transition-colors",
                        MaterialIcon { name: "{meta.icon_name}", size: 20, color: MaterialIconColor::Custom(meta.icon_color.to_string()) }
                    }
                    div { class: "flex flex-col gap-0.5",
                        span { class: "line-clamp-1", "{filename}" }
                        span { class: "text-[9px] text-slate-400 uppercase tracking-tighter font-black", "Click to Download" }
                    }
                }
            }
        },
        None => rsx! {
            div { class: "space-y-2",
                div { class: "rounded-2xl border border-slate-200 bg-white px-4 py-6 text-sm text-slate-500 shadow-sm", "File preview unavailable." }
                div { class: "text-[10px] font-semibold text-slate-500 px-1 break-all", "{filename}" }
            }
        },
    }
}
