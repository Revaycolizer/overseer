pub struct AttachmentMeta {
    pub icon_name: &'static str,
    pub icon_color: &'static str,
}

pub fn get_attachment_meta(content_type: &str, filename: &str) -> AttachmentMeta {
    let is_image = content_type.starts_with("image/");
    let icon_name = if is_image { "image" } 
    else if content_type.contains("pdf") { "picture_as_pdf" }
    else if content_type.contains("spreadsheet") || content_type.contains("excel") || filename.ends_with(".xlsx") || filename.ends_with(".xls") || filename.ends_with(".csv") { "table_view" }
    else if content_type.contains("word") || filename.ends_with(".docx") || filename.ends_with(".doc") { "description" }
    else { "insert_drive_file" };

    let icon_color = match icon_name {
        "picture_as_pdf" => "text-red-500",
        "table_view" => "text-emerald-600",
        "description" => "text-blue-600",
        _ => "text-slate-400",
    };

    AttachmentMeta { icon_name, icon_color }
}
