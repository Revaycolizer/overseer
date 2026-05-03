use std::path::PathBuf;
use serde_json::Value;

pub fn generate_csv(
    dir: &PathBuf,
    prefix: &str,
    headers: &[String],
    data: &Vec<Value>,
) -> Result<String, String> {
    let filename = format!("{}.csv", prefix);
    let path = dir.join(&filename);
    let mut wtr = csv::Writer::from_path(path).map_err(|e| e.to_string())?;

    wtr.write_record(headers).map_err(|e| e.to_string())?;
    for row in data {
        if let Some(arr) = row.as_array() {
            let record: Vec<String> = arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
            wtr.write_record(record).map_err(|e| e.to_string())?;
        }
    }
    wtr.flush().map_err(|e| e.to_string())?;
    Ok(filename)
}
