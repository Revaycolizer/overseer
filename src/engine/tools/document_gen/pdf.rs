use std::path::PathBuf;
use serde_json::Value;
use genpdf::Element;
use super::utils::find_system_font;

pub fn generate_pdf(
    dir: &PathBuf,
    prefix: &str,
    headers: Option<&[String]>,
    data: Option<&Vec<Value>>,
    content: Option<&str>,
) -> Result<String, String> {
    let filename = format!("{}.pdf", prefix);
    let path = dir.join(&filename);
    let font = find_system_font()?;
    let mut doc = genpdf::Document::new(font);
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    if let Some(text) = content {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { doc.push(genpdf::elements::Break::new(1)); continue; }
            doc.push(genpdf::elements::Paragraph::new(trimmed));
        }
    }

    if let (Some(h), Some(_d)) = (headers, data) {
        let mut table = genpdf::elements::TableLayout::new(vec![1; h.len()]);
        let mut hr = table.row();
        for head in h { hr.push_element(genpdf::elements::Text::new(head).styled(genpdf::style::Style::new().bold())); }
        hr.push().map_err(|e| e.to_string())?;
        doc.push(table);
    }

    doc.render_to_file(&path).map_err(|e| e.to_string())?;
    Ok(filename)
}
