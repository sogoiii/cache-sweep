use super::ScanResult;
use tokio::sync::mpsc;

const BATCH_SIZE: usize = 50;

pub struct ResultBatcher {
    buffer: Vec<ScanResult>,
    tx: mpsc::UnboundedSender<Vec<ScanResult>>,
}

impl ResultBatcher {
    pub fn new(tx: mpsc::UnboundedSender<Vec<ScanResult>>) -> Self {
        Self {
            buffer: Vec::with_capacity(BATCH_SIZE),
            tx,
        }
    }

    pub fn add(&mut self, result: ScanResult) {
        self.buffer.push(result);
        if self.buffer.len() >= BATCH_SIZE {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let batch = std::mem::take(&mut self.buffer);
            // Use .send() for unbounded channel (not blocking_send)
            let _ = self.tx.send(batch);
        }
    }
}

impl Drop for ResultBatcher {
    fn drop(&mut self) {
        self.flush();
    }
}
