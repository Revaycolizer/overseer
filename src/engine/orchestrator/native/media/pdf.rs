// src/engine/orchestrator/native/media/pdf.rs
use std::path::Path;

/// Extracts text from a PDF file natively.
pub fn extract_text(path: &Path) -> anyhow::Result<String> {
    tracing::info!(?path, "Extracting PDF");

    // pdf-extract uses a very simple API to get all text
    let out = pdf_extract::extract_text(path)?;

    if out.trim().is_empty() {
        anyhow::bail!("PDF extraction returned empty text (possibly scanned or encrypted)");
    }

    Ok(out)
}
