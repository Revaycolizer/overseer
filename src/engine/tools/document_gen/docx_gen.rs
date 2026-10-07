use std::path::PathBuf;

pub fn generate_docx(dir: &PathBuf, prefix: &str, content: &str) -> Result<String, String> {
    let filename = format!("{}.docx", prefix);
    let path = dir.join(&filename);
    let mut docx = docx_rs::Docx::new();

    for line in content.lines() {
        docx = docx
            .add_paragraph(docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text(line)));
    }

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    docx.build().pack(file).map_err(|e| e.to_string())?;
    Ok(filename)
}
