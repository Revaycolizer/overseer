use std::path::PathBuf;

pub struct AuditLog {
    path: PathBuf,
}

impl AuditLog {
    pub fn new(session_dir: PathBuf) -> Self {
        Self { path: session_dir.join("audit_log.jsonl") }
    }

    pub async fn write(&self, entry: &serde_json::Value) {
        let line = format!("{}\n", entry);
        let _ = tokio::fs::OpenOptions::new()
            .create(true).append(true).open(&self.path).await
            .and_then(|mut f| tokio::io::AsyncWriteExt::write_all(&mut f, line.as_bytes()));
    }
}
