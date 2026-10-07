use rust_xlsxwriter::*;
use serde_json::Value;
use std::path::PathBuf;

pub fn generate_xlsx(
    dir: &PathBuf,
    prefix: &str,
    headers: &[String],
    data: &Vec<Value>,
) -> Result<String, String> {
    let filename = format!("{}.xlsx", prefix);
    let path = dir.join(&filename);
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (i, h) in headers.iter().enumerate() {
        worksheet
            .write_string(0, i as u16, h)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, row) in data.iter().enumerate() {
        if let Some(arr) = row.as_array() {
            for (col_idx, val) in arr.iter().enumerate() {
                worksheet
                    .write_string(
                        (row_idx + 1) as u32,
                        col_idx as u16,
                        val.as_str().unwrap_or(""),
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    workbook
        .save(path.to_str().unwrap())
        .map_err(|e| e.to_string())?;
    Ok(filename)
}
