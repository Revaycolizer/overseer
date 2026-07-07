use crate::db::Database;
use super::extraction::{extract_local_file_paths_from_content, is_image_file_path};

pub async fn attach_assistant_files_from_content(
    db: &Database,
    message_id: i64,
    content: &str,
) -> anyhow::Result<()> {
    let paths = extract_local_file_paths_from_content(content);
    if paths.is_empty() {
        return Ok(());
    }

    for path in paths {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let file_path = path.display().to_string();
        let size = std::fs::metadata(&path)
            .map(|meta| meta.len() as i64)
            .unwrap_or(0);

        let content_type = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .essence_str()
            .to_string();

        let is_image = is_image_file_path(&path);
        let mut extracted_content = None;

        if !is_image {
            let extension = path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase());
            match extension.as_deref() {
                Some("pdf") => {
                    if let Ok(text) = crate::engine::orchestrator::native::media::pdf::extract_text(&path) {
                        if !text.trim().is_empty() {
                            extracted_content = Some(text);
                        }
                    }
                }
                Some("docx") => {
                    if let Ok(text) = crate::engine::orchestrator::native::media::word::extract_text(&path) {
                        if !text.trim().is_empty() {
                            extracted_content = Some(text);
                        }
                    }
                }
                _ => {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        if !text.trim().is_empty() {
                            extracted_content = Some(text);
                        }
                    }
                }
            }
        }

        let attachment_id = super::add_attachment(
            db,
            message_id,
            &filename,
            &file_path,
            &content_type,
            size,
        )
        .await?;

        if let Some(text) = extracted_content {
            let _ = super::update_attachment_content(db, attachment_id, &text).await;
        }
    }

    Ok(())
}
