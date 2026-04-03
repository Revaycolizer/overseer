use std::path::PathBuf;
use serde_json::json;
use chrono;
use rayon::prelude::*;
use csv;
use rust_xlsxwriter;
use docx_rs::*;
use genpdf;
use genpdf::Element;
use std::fs;

// Moved from src/engine/tools/mod.rs
pub async fn native_document_writer(args: &serde_json::Value) -> Result<String, String> {
    let data_val = args.get("data");
    
    // Smart Content Resolution: Try 'content' first, fallback to 'sections' array
    let mut resolved_content = args.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
    if resolved_content.is_none() {
        if let Some(sections) = args.get("sections").and_then(|v| v.as_array()) {
            let mut combined = String::new();
            for s in sections {
                if let Some(title) = s.get("title").and_then(|v| v.as_str()) {
                    combined.push_str(&format!("## {}\n", title));
                }
                if let Some(text) = s.get("content").and_then(|v| v.as_str()) {
                    combined.push_str(&format!("{}\n\n", text));
                }
            }
            if !combined.is_empty() {
                resolved_content = Some(combined);
            }
        }
    }
    let content_val = resolved_content.as_deref();

    let formats_val = args.get("formats").and_then(|v| v.as_array());
    
    if data_val.is_none() && content_val.is_none() {
        return Ok(json!({
            "status": "success",
            "message": "Tool document_writer initialized. Please provide 'content' or 'data'.",
            "artifacts": []
        }).to_string());
    }

    let data = data_val.and_then(|v| v.as_array());
    let formats: Vec<String> = if let Some(arr) = formats_val {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    } else {
        // Handle case where agent sends a single string instead of an array
        if let Some(s) = args.get("formats").and_then(|v| v.as_str()) {
            vec![s.to_string()]
        } else if let Some(s) = args.get("type").and_then(|v| v.as_str()) {
            // Also map 'type' to format (e.g. type: PDF)
            vec![s.to_lowercase()]
        } else {
            vec!["pdf".to_string()] 
        }
    };

    // Smart Filename Resolution: Try 'filename' first, fallback to 'title'
    let base_filename = args
        .get("filename")
        .or_else(|| args.get("title"))
        .and_then(|f| f.as_str())
        .unwrap_or("overseer_report")
        .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let export_dir = PathBuf::from("assets/exports");
    
    // Ensure export directory exists
    if let Err(e) = fs::create_dir_all(&export_dir) {
        return Err(format!("Failed to create export directory: {}", e));
    }

    let file_prefix = format!("{}_{}", base_filename, timestamp);

    let mut artifacts = Vec::new();
    let mut errors = Vec::new();

    // Prepare headers if data is present
    let headers: Vec<String> = if let Some(d) = data {
        if let Some(first) = d.first() {
            if let Some(obj) = first.as_object() {
                obj.keys().cloned().collect()
            } else {
                vec!["Value".to_string()]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    use rayon::prelude::*;

    let results: Vec<Result<String, String>> = if formats.len() > 1 {
        // PARALLEL path (Rayon)
        formats.par_iter().map(|format| {
            generate_single_format(format, &export_dir, &file_prefix, &headers, data, content_val)
        }).collect()
    } else {
        // SYNCHRONOUS path
        formats.iter().map(|format| {
            generate_single_format(format, &export_dir, &file_prefix, &headers, data, content_val)
        }).collect()
    };

    for (idx, res) in results.into_iter().enumerate() {
        let format = &formats[idx];
        match res {
            Ok(filename) => {
                let label = match format.as_str() {
                    "csv" => "CSV Export",
                    "xlsx" => "Excel Spreadsheet",
                    "docx" => "Word Document",
                    "pdf" => "PDF Report",
                    _ => "Export",
                };
                artifacts.push(json!({
                    "path": format!("assets/exports/{}", filename),
                    "kind": format,
                    "label": label
                }));
            }
            Err(e) => errors.push(e),
        }
    }

    if artifacts.is_empty() && !errors.is_empty() {
        return Err(format!("Document generation failed: {}", errors.join("; ")));
    }

    Ok(json!({
        "status": "success",
        "message": format!("Successfully generated {} document(s).", artifacts.len()),
        "artifacts": artifacts,
        "errors": if errors.is_empty() { None } else { Some(errors) }
    }).to_string())
}

fn generate_single_format(
    format: &str,
    dir: &PathBuf,
    prefix: &str,
    headers: &[String],
    data: Option<&Vec<serde_json::Value>>,
    content: Option<&str>,
) -> Result<String, String> {
    match format {
        "csv" => {
            if let Some(d) = data {
                generate_csv(dir, prefix, headers, d)
            } else {
                Err("CSV requires 'data' array.".to_string())
            }
        },
        "xlsx" => {
            if let Some(d) = data {
                generate_xlsx(dir, prefix, headers, d)
            } else {
                Err("Excel requires 'data' array.".to_string())
            }
        },
        "docx" => {
            generate_docx(dir, prefix, if headers.is_empty() { None } else { Some(headers) }, data, content)
        },
        "pdf" => {
            generate_pdf(dir, prefix, if headers.is_empty() { None } else { Some(headers) }, data, content)
        },
        _ => Err(format!("Unsupported format: {}", format)),
    }
}

fn generate_csv(
    dir: &PathBuf,
    prefix: &str,
    headers: &[String],
    data: &[serde_json::Value],
) -> Result<String, String> {
    let filename = format!("{}.csv", prefix);
    let path = dir.join(&filename);
    let mut wtr = csv::Writer::from_path(&path).map_err(|e| e.to_string())?;

    wtr.write_record(headers).map_err(|e| e.to_string())?;
    for row in data {
        let mut record = Vec::new();
        for h in headers {
            record.push(
                row.get(h)
                    .map(|v| {
                        if v.is_string() {
                            v.as_str().unwrap().to_string()
                        } else {
                            v.to_string()
                        }
                    })
                    .unwrap_or_default(),
            );
        }
        wtr.write_record(&record).map_err(|e| e.to_string())?;
    }
    wtr.flush().map_err(|e| e.to_string())?;
    Ok(filename)
}

fn generate_xlsx(
    dir: &PathBuf,
    prefix: &str,
    headers: &[String],
    data: &[serde_json::Value],
) -> Result<String, String> {
    let filename = format!("{}.xlsx", prefix);
    let path = dir.join(&filename);
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (i, h) in headers.iter().enumerate() {
        worksheet
            .write_string(0, i as u16, h)
            .map_err(|e| e.to_string())?;
    }

    for (r, row) in data.iter().enumerate() {
        for (c, h) in headers.iter().enumerate() {
            let val = row
                .get(h)
                .map(|v| {
                    if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_default();
            worksheet
                .write_string((r + 1) as u32, c as u16, &val)
                .map_err(|e| e.to_string())?;
        }
    }

    workbook.save(&path).map_err(|e| e.to_string())?;
    Ok(filename)
}

fn generate_docx(
    dir: &PathBuf,
    prefix: &str,
    headers: Option<&[String]>,
    data: Option<&Vec<serde_json::Value>>,
    content: Option<&str>,
) -> Result<String, String> {
    let filename = format!("{}.docx", prefix);
    let path = dir.join(&filename);

    let mut doc = Docx::new();

    if let Some(text) = content {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                doc = doc.add_paragraph(Paragraph::new());
                continue;
            }

            let mut p = Paragraph::new();
            let (level, clean_text) = if trimmed.starts_with("### ") {
                (3, trimmed.trim_start_matches("### ").trim())
            } else if trimmed.starts_with("## ") {
                (2, trimmed.trim_start_matches("## ").trim())
            } else if trimmed.starts_with("# ") {
                (1, trimmed.trim_start_matches("# ").trim())
            } else {
                (0, trimmed)
            };

            if level > 0 {
                let size = match level { 1 => 48, 2 => 36, 3 => 28, _ => 24 };
                p = p.add_run(Run::new().add_text(clean_text).size(size).bold());
            } else {
                let (is_bold, final_text) = if trimmed.starts_with("**") && trimmed.ends_with("**") {
                    (true, trimmed.trim_matches('*').trim())
                } else {
                    (false, trimmed)
                };
                let run = Run::new().add_text(final_text);
                p = p.add_run(if is_bold { run.bold() } else { run });
            }
            doc = doc.add_paragraph(p);
        }
    }

    if let (Some(h_list), Some(d_list)) = (headers, data) {
        let mut table = Table::new(vec![]);
        let mut cells = Vec::new();
        for h in h_list {
            cells.push(
                TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(h).bold())),
            );
        }
        table = table.add_row(TableRow::new(cells));

        for row in d_list {
            let mut row_cells = Vec::new();
            for h in h_list {
                let val = row.get(h).map(|v| if v.is_string() { v.as_str().unwrap().to_string() } else { v.to_string() }).unwrap_or_default();
                row_cells.push(TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(val))));
            }
            table = table.add_row(TableRow::new(row_cells));
        }
        doc = doc.add_table(table);
    }

    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    doc.build().pack(file).map_err(|e| e.to_string())?;

    Ok(filename)
}

fn generate_pdf(
    dir: &PathBuf,
    prefix: &str,
    headers: Option<&[String]>,
    data: Option<&Vec<serde_json::Value>>,
    content: Option<&str>,
) -> Result<String, String> {
    let filename = format!("{}.pdf", prefix);
    let path = dir.join(&filename);

    let font_family = find_system_font()?;
    let mut doc = genpdf::Document::new(font_family);

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    if let Some(text) = content {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                doc.push(genpdf::elements::Break::new(1));
                continue;
            }

            // Headers
            let (level, clean_text) = if trimmed.starts_with("### ") {
                (3, trimmed.trim_start_matches("### ").trim())
            } else if trimmed.starts_with("## ") {
                (2, trimmed.trim_start_matches("## ").trim())
            } else if trimmed.starts_with("# ") {
                (1, trimmed.trim_start_matches("# ").trim())
            } else {
                (0, trimmed)
            };

            if level > 0 {
                let size = match level { 1 => 20, 2 => 16, 3 => 14, _ => 12 };
                doc.push(genpdf::elements::Paragraph::new(clean_text)
                    .styled(genpdf::style::Style::new().with_font_size(size).bold()));
                doc.push(genpdf::elements::Break::new(0.5));
                continue;
            }

            // Bullet Points
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let bullet_text = trimmed[2..].trim();
                let mut p = genpdf::elements::Paragraph::new(format!("  • {}", bullet_text));
                doc.push(p);
                continue;
            }

            // Bold Lines (Full Line)
            let (is_bold, final_text) = if trimmed.starts_with("**") && trimmed.ends_with("**") {
                (true, trimmed.trim_matches('*').trim())
            } else {
                (false, trimmed)
            };

            let mut p = genpdf::elements::Paragraph::new(final_text);
            if is_bold {
                doc.push(p.styled(genpdf::style::Style::new().bold()));
            } else {
                doc.push(p);
            }
        }
        doc.push(genpdf::elements::Break::new(1));
    }

    if let (Some(h_list), Some(d_list)) = (headers, data) {
        let mut table = genpdf::elements::TableLayout::new(vec![1; h_list.len()]);
        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, true));

        let mut header_row = table.row();
        for h in h_list {
            header_row.push_element(genpdf::elements::Text::new(h).styled(genpdf::style::Style::new().bold()));
        }
        header_row.push().map_err(|e| e.to_string())?;

        for row in d_list {
            let mut data_row = table.row();
            for h in h_list {
                let val = row.get(h).map(|v| if v.is_string() { v.as_str().unwrap().to_string() } else { v.to_string() }).unwrap_or_default();
                data_row.push_element(genpdf::elements::Text::new(val));
            }
            data_row.push().map_err(|e| e.to_string())?;
        }
        doc.push(table);
    }
    doc.render_to_file(&path).map_err(|e| e.to_string())?;

    Ok(filename)
}

fn find_system_font() -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, String> {
    // 1. Try Bundled Assets First (Prioritized for consistency)
    if let Ok(f) = genpdf::fonts::from_files("./assets/fonts", "Roboto", None) {
        return Ok(f);
    }
    
    // 2. Extra Robust Fallback: try relative path for dev environments
    if let Ok(f) = genpdf::fonts::from_files("assets/fonts", "Roboto", None) {
        return Ok(f);
    }
    
    // 2. Platform Heuristics for Cross-Platform portability (Windows, Mac, Linux)
    let paths = if cfg!(target_os = "windows") {
        vec!["C:\\Windows\\Fonts\\arial.ttf", "C:\\Windows\\Fonts\\times.ttf"]
    } else if cfg!(target_os = "macos") {
        vec!["/Library/Fonts/Arial.ttf", "/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/Keyboard.ttf"]
    } else {
        // Common Linux font locations
        vec![
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/roboto/Roboto-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf"
        ]
    };

    for path in paths {
        let p = std::path::Path::new(path);
        if p.exists() {
            if let (Some(dir), Some(stem)) = (p.parent(), p.file_stem()) {
                if let Some(name) = stem.to_str() {
                    if let Ok(f) = genpdf::fonts::from_files(dir, name, None) {
                        return Ok(f);
                    }
                }
            }
        }
    }

    Err("No suitable system font found for PDF generation. Please ensure at least one TrueType font (DejaVu, Arial, or Roboto) is installed.".to_string())
}
