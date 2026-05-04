// src/engine/orchestrator/native/media/word.rs
use std::path::Path;
use docx_rs::*;

/// Extracts text from a Word (.docx) file natively.
pub fn extract_text(path: &Path) -> anyhow::Result<String> {
    println!("📝 [Native] Extracting Word: {:?}", path);
    
    let file = std::fs::read(path)?;
    let docx = read_docx(&file)?;
    
    let mut full_text = String::new();
    
    // Simple traversal of the document elements
    for child in docx.document.children {
        if let DocumentChild::Paragraph(p) = child {
            for child in p.children {
                if let ParagraphChild::Run(r) = child {
                    for child in r.children {
                        if let RunChild::Text(t) = child {
                            full_text.push_str(&t.text);
                        }
                    }
                }
            }
            full_text.push('\n');
        }
    }
    
    if full_text.trim().is_empty() {
        anyhow::bail!("Word extraction returned empty text");
    }
    
    Ok(full_text)
}
