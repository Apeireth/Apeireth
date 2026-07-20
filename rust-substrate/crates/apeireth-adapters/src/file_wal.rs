//! File WAL adapter (借鉴 DeltaMemory WAL)

use async_trait::async_trait;
use apeireth_ports::{WalSink, PortError};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct FileWalSink {
    file: Arc<Mutex<std::fs::File>>,
    sequence: Arc<Mutex<u64>>,
    path: std::path::PathBuf,
}

impl FileWalSink {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, PortError> {
        use std::fs::OpenOptions;
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PortError::Io(e.to_string()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| PortError::Io(e.to_string()))?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            sequence: Arc::new(Mutex::new(0)),
            path,
        })
    }
}

#[async_trait]
impl WalSink for FileWalSink {
    async fn append(&self, payload: &[u8]) -> Result<u64, PortError> {
        use std::io::Write;
        let mut seq = self.sequence.lock();
        let mut file = self.file.lock();
        *seq += 1;
        writeln!(file, "{} {}", seq, std::str::from_utf8(payload).unwrap_or("<bin>"))
            .map_err(|e| PortError::Io(e.to_string()))?;
        file.flush().map_err(|e| PortError::Io(e.to_string()))?;
        Ok(*seq)
    }

    async fn replay<F>(&self, mut handler: F) -> Result<u64, PortError>
    where
        F: FnMut(&[u8]) -> Result<(), PortError> + Send,
    {
        use std::io::{BufRead, BufReader};
        if !self.path.exists() {
            return Ok(0);
        }
        let file = std::fs::File::open(&self.path).map_err(|e| PortError::Io(e.to_string()))?;
        let reader = BufReader::new(file);
        let mut count = 0u64;
        for line in reader.lines().map_while(|r| r.ok()) {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                handler(parts[1].as_bytes())?;
                count += 1;
            }
        }
        Ok(count)
    }
}