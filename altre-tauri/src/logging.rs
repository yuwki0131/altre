use crate::options::ensure_parent_dir;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// GUI デバッグログを JSON Lines 形式で出力するロガー
pub struct DebugLogger {
    path: PathBuf,
}

impl DebugLogger {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        ensure_parent_dir(&path)?;
        Ok(Self { path })
    }

    pub fn log_event<T: Serialize>(&self, tag: &str, payload: &T) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let record = serde_json::json!({
            "tag": tag,
            "ts": timestamp_ms(),
            "payload": payload
        });
        let line = serde_json::to_string(&record)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis())
        .unwrap_or_default()
}
