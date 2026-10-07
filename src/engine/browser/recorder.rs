use std::path::PathBuf;
use tokio::sync::broadcast;

pub struct FrameRecorder {
    pub frames_dir: PathBuf,
    pub broadcast_tx: broadcast::Sender<Vec<u8>>,
}

impl FrameRecorder {
    pub fn new(frames_dir: PathBuf) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        Self {
            frames_dir,
            broadcast_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.broadcast_tx.subscribe()
    }
}
