use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DebugLogger {
    file: Mutex<File>,
    last_state_hash: Mutex<Option<u64>>,
    log_path: PathBuf,
}

impl DebugLogger {
    pub fn new(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(Self {
            file: Mutex::new(file),
            last_state_hash: Mutex::new(None),
            log_path: path.to_path_buf(),
        })
    }

    pub fn log_state(&self, state: DebugState) {
        let state_hash = hash_record(&state);
        {
            let mut guard = self
                .last_state_hash
                .lock()
                .expect("debug logger last_state_hash poisoned");
            if guard.map(|hash| hash == state_hash).unwrap_or(false) {
                return;
            }
            *guard = Some(state_hash);
        }
        let record = DebugRecord::State(state);
        if let Err(err) = self.write_record(&record) {
            eprintln!("[debug log] failed to write state: {}", err);
        }
    }

    pub fn log_event(&self, event: DebugEvent) {
        let record = DebugRecord::Event(event);
        if let Err(err) = self.write_record(&record) {
            eprintln!("[debug log] failed to write event: {}", err);
        }
    }

    pub fn log_message(&self, level: &'static str, message: String) {
        let record = DebugRecord::Message(DebugMessage {
            timestamp: current_timestamp(),
            level: level.to_string(),
            message,
        });
        if let Err(err) = self.write_record(&record) {
            eprintln!("[debug log] failed to write message: {}", err);
        }
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    fn write_record(&self, record: &DebugRecord) -> std::io::Result<()> {
        let mut file = self.file.lock().expect("debug logger file poisoned");
        let json = serde_json::to_string(record)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }
}

#[derive(Serialize, Hash)]
#[serde(tag = "record_type", rename_all = "snake_case")]
enum DebugRecord {
    State(DebugState),
    Event(DebugEvent),
    Message(DebugMessage),
}

#[derive(Serialize, Hash)]
pub struct DebugState {
    pub timestamp: String,
    pub status_label: String,
    pub is_modified: bool,
    pub buffer_len: usize,
    pub buffer_sample: String,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub displayed_lines: usize,
    pub minibuffer_prompt: String,
    pub minibuffer_input: String,
    pub minibuffer_mode: String,
    pub minibuffer_message: String,
    pub minibuffer_completions: Vec<String>,
}

#[derive(Serialize, Hash)]
pub struct DebugEvent {
    pub timestamp: String,
    pub event: String,
    pub detail: String,
}

#[derive(Serialize, Hash)]
struct DebugMessage {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

pub fn current_timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{:09}", duration.as_secs(), duration.subsec_nanos()),
        Err(_) => "0.0".to_string(),
    }
}

fn hash_record<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn truncate_for_log(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else if max_len <= 1 {
        "...".to_string()
    } else {
        let mut truncated = text[..max_len - 1].to_string();
        truncated.push('…');
        truncated
    }
}
