// src/engine/orchestrator/native/media/image.rs
use std::path::Path;
use std::process::Command;

/// Checks if Tesseract OCR is installed and available in the system PATH.
pub fn is_ocr_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .is_ok()
}

/// Extracts text from an image using Tesseract OCR.
pub fn extract_text(path: &Path) -> anyhow::Result<String> {
    if !is_ocr_available() {
        anyhow::bail!("Tesseract OCR not found in system PATH");
    }

    tracing::info!(?path, "Performing OCR on image");
    
    // Run tesseract: tesseract [input] stdout
    let output = Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tesseract error: {}", err);
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    
    if text.trim().is_empty() {
        anyhow::bail!("OCR returned empty text");
    }

    Ok(text)
}
