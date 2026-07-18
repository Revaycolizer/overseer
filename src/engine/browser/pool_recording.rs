use super::pool::BrowserPool;
use super::recorder::FrameRecorder;
use tokio::sync::broadcast;

impl BrowserPool {
    pub async fn start_recording(&self, id: i64, quality: u8) -> anyhow::Result<broadcast::Receiver<Vec<u8>>> {
        let inner = self.inner.lock().await;
        let session = inner.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("Session {id} not found"))?;

        if let Some(ref existing) = session.recorder {
            return Ok(existing.subscribe());
        }

        let frames_dir = session.profile_path.join("recording").join("frames");
        std::fs::create_dir_all(&frames_dir)?;
        let recorder = FrameRecorder::new(frames_dir);
        let rx = recorder.subscribe();

        if let Some(ref cdp) = session.cdp {
            let bg_cdp = cdp.clone();
            tokio::spawn(async move {
                let _ = bg_cdp.call("Page.startScreencast", serde_json::json!({
                    "format": "jpeg", "quality": quality, "maxWidth": 1280, "everyNthFrame": 1,
                })).await;
            });
        }

        session.recorder = Some(recorder);
        Ok(rx)
    }

    pub async fn stop_recording(&self, id: i64) -> anyhow::Result<()> {
        let inner = self.inner.lock().await;
        if let Some(session) = inner.sessions.get(&id) {
            if let Some(ref cdp) = session.cdp {
                let _ = cdp.call("Page.stopScreencast", serde_json::json!({})).await;
            }
        }
        Ok(())
    }
}
