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
    
    if data_val.is_none() || (data_val.is_some() && data_val.unwrap().is_array() && data_val.unwrap().as_array().unwrap().is_empty()) {
        return Ok(json!({
            "status": "success",
            "message": "Tool document_writer initialized and ready. Provide a 'data' array to generate reports.",
            "artifacts": []
        }).to_string());
    }

    let data = data_val.unwrap().as_array().ok_or_else(|| {
        "Invalid 'data' format. Expected an array of objects.".to_string()
    })?;

    let base_filename = args
        .get("filename")
        .and_then(|f| f.as_str())
        .unwrap_or("overseer_export")
        .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let export_dir = PathBuf::from("assets/exports");
    
    // Ensure export directory exists
    if let Err(e) = fs::create_dir_all(&export_dir) {
        return Err(format!("Failed to create export directory: {}", e));
    }

    let file_prefix = format!("{}_{}", base_filename, timestamp);

    // Prepare headers from the first object
    let headers: Vec<String> = if let Some(first) = data.first() {
        if let Some(obj) = first.as_object() {
            obj.keys().cloned().collect()
        } else {
            vec!["Value".to_string()]
        }
    } else {
        vec![]
    };

    let data_cloned = data.clone();
    let headers_cloned = headers.clone();
    let file_prefix_cloned = file_prefix.clone();
    let export_dir_cloned = export_dir.clone();

    // Perform parallel generation using rayon
    let results: Vec<Result<String, String>> = rayon::join(
        || {
            rayon::join(
                || {
                    generate_csv(
                        &export_dir_cloned,
                        &file_prefix_cloned,
                        &headers_cloned,
                        &data_cloned,
                    )
                },
                || {
                    generate_xlsx(
                        &export_dir_cloned,
                        &file_prefix_cloned,
                        &headers_cloned,
                        &data_cloned,
                    )
                },
            )
        },
        || {
            rayon::join(
                || {
                    generate_docx(
                        &export_dir_cloned,
                        &file_prefix_cloned,
                        &headers_cloned,
                        &data_cloned,
                    )
                },
                || {
                    generate_pdf(
                        &export_dir_cloned,
                        &file_prefix_cloned,
                        &headers_cloned,
                        &data_cloned,
                    )
                },
            )
        },
    )
    .into_results();

    let mut artifacts = Vec::new();
    let mut errors = Vec::new();

    for (i, res) in results.into_iter().enumerate() {
        match res {
            Ok(filename) => {
                let kind = match i {
                    0 => "csv",
                    1 => "xlsx",
                    2 => "docx",
                    3 => "pdf",
                    _ => "unknown",
                };
                let label = match i {
                    0 => "CSV Export",
                    1 => "Excel Spreadsheet",
                    2 => "Word Document",
                    3 => "PDF Report",
                    _ => "Export",
                };
                artifacts.push(json!({
                    "path": format!("assets/exports/{}", filename),
                    "kind": kind,
                    "label": label
                }));
            }
            Err(e) => errors.push(e),
        }
    }

    if artifacts.is_empty() && !errors.is_empty() {
        return Err(format!("All document generation tasks failed: {}", errors.join("; ")));
    }

    Ok(json!({
        "status": "success",
        "message": format!("Successfully generated {} document(s) in parallel.", artifacts.len()),
        "artifacts": artifacts,
        "errors": if errors.is_empty() { None } else { Some(errors) }
    }).to_string())
}

// Helper trait to flatten nested join results into a simple Vec
trait JoinExt {
    fn into_results(self) -> Vec<Result<String, String>>;
}

impl JoinExt
    for (
        (Result<String, String>, Result<String, String>),
        (Result<String, String>, Result<String, String>),
    )
{
    fn into_results(self) -> Vec<Result<String, String>> {
        vec![self.0 .0, self.0 .1, self.1 .0, self.1 .1]
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
    headers: &[String],
    data: &[serde_json::Value],
) -> Result<String, String> {
    let filename = format!("{}.docx", prefix);
    let path = dir.join(&filename);

    let mut doc = Docx::new();
    doc = doc.add_paragraph(
        Paragraph::new().add_run(
            Run::new()
                .add_text(format!("Export: {}", prefix))
                .size(24)
                .bold(),
        ),
    );

    let mut table = Table::new(vec![]);

    // Header row
    let mut cells = Vec::new();
    for h in headers {
        cells.push(
            TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(h).bold())),
        );
    }
    table = table.add_row(TableRow::new(cells));

    // Data rows
    for row in data {
        let mut row_cells = Vec::new();
        for h in headers {
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
            row_cells.push(
                TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(val))),
            );
        }
        table = table.add_row(TableRow::new(row_cells));
    }

    doc = doc.add_table(table);

    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    doc.build().pack(file).map_err(|e| e.to_string())?;

    Ok(filename)
}

fn generate_pdf(
    dir: &PathBuf,
    prefix: &str,
    headers: &[String],
    data: &[serde_json::Value],
) -> Result<String, String> {
    let filename = format!("{}.pdf", prefix);
    let path = dir.join(&filename);

    let font_family = genpdf::fonts::from_files("./assets/fonts", "Roboto", None)
        .map_err(|e| format!("Could not load fonts: {}", e))?;
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(format!("Export: {}", prefix));

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);

    doc.push(
        genpdf::elements::Text::new(format!("Export Generated: {}", prefix))
            .styled(genpdf::style::Style::new().with_font_size(18).bold()),
    );
    doc.push(genpdf::elements::Break::new(1));

    let mut table = genpdf::elements::TableLayout::new(vec![1; headers.len()]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, true));

    // Header
    let mut header_row = table.row();
    for h in headers {
        header_row.push_element(
            genpdf::elements::Text::new(h).styled(genpdf::style::Style::new().bold()),
        );
    }
    header_row.push().map_err(|e| e.to_string())?;

    // Data
    for row in data {
        let mut data_row = table.row();
        for h in headers {
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
            data_row.push_element(genpdf::elements::Text::new(val));
        }
        data_row.push().map_err(|e| e.to_string())?;
    }

    doc.push(table);
    doc.render_to_file(&path).map_err(|e| e.to_string())?;

    Ok(filename)
}
