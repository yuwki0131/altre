use crate::keymap::KeySequencePayload;
use crate::logging::DebugLogger;
use crate::options::BackendOptions;
use crate::snapshot::EditorSnapshot;
use altre::error::{AltreError, Result};
use altre::Backend;
use crossterm::event::{KeyCode as CrosstermKeyCode, KeyEvent, KeyModifiers as CrosstermModifiers};
use serde::Serialize;
use serde_json::json;

/// GUI から Rust バックエンドを操作するコントローラー
pub struct BackendController {
    backend: Backend,
    logger: Option<DebugLogger>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SaveResponse {
    pub success: bool,
    pub message: Option<String>,
}

impl BackendController {
    pub fn new(options: BackendOptions) -> Result<Self> {
        let backend = Backend::new()?;
        let logger = match options.resolve_log_path() {
            Some(path) => Some(DebugLogger::new(path).map_err(log_error)?),
            None => None,
        };
        Ok(Self { backend, logger })
    }

    pub fn snapshot(&mut self) -> Result<EditorSnapshot> {
        self.backend.process_minibuffer_timer();
        let snapshot = self.create_snapshot();
        if let Some(snapshot) = snapshot.as_ref().ok() {
            self.log_event("snapshot", snapshot)?;
        }
        snapshot
    }

    pub fn handle_key_events(&mut self, events: &[KeyEvent]) -> Result<EditorSnapshot> {
        for event in events {
            self.backend.handle_key_event(*event)?;
        }
        let description: Vec<String> = events.iter().map(describe_key_event).collect();
        self.log_event("key_sequence", &description)?;
        self.snapshot()
    }

    pub fn handle_serialized_keys(
        &mut self,
        payload: KeySequencePayload,
    ) -> Result<EditorSnapshot> {
        let events = payload.into_key_events()?;
        self.handle_key_events(&events)
    }

    pub fn open_file(&mut self, path: &str) -> Result<EditorSnapshot> {
        self.backend.open_file(path)?;
        self.log_event("open_file", &json!({ "path": path }))?;
        self.snapshot()
    }

    pub fn save_active_buffer(&mut self) -> Result<SaveResponse> {
        Err(AltreError::Application(
            "GUI 保存処理は未実装です".to_string(),
        ))
    }

    pub fn shutdown(&mut self) {
        self.backend.shutdown();
    }

    fn create_snapshot(&mut self) -> Result<EditorSnapshot> {
        let metadata = self.backend.render_metadata();
        let view = self.backend.render_view();
        let text = view.editor.to_string();
        let cursor = *view.editor.cursor();
        let snapshot = EditorSnapshot::new(&text, &cursor, &metadata, view.minibuffer);
        Ok(snapshot)
    }

    fn log_event<T: Serialize>(&self, tag: &str, payload: &T) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger
                .log_event(tag, payload)
                .map_err(|err| AltreError::Application(format!("デバッグログ出力に失敗しました: {err}")))?;
        }
        Ok(())
    }
}

fn describe_key_event(event: &KeyEvent) -> String {
    let mut parts = Vec::new();

    if event.modifiers.contains(CrosstermModifiers::CONTROL) {
        parts.push("C".to_string());
    }
    if event.modifiers.contains(CrosstermModifiers::ALT) {
        parts.push("M".to_string());
    }
    if event.modifiers.contains(CrosstermModifiers::SHIFT) {
        parts.push("S".to_string());
    }

    let key = match event.code {
        CrosstermKeyCode::Char(ch) => ch.to_string(),
        CrosstermKeyCode::Enter => "Enter".to_string(),
        CrosstermKeyCode::Backspace => "Backspace".to_string(),
        CrosstermKeyCode::Delete => "Delete".to_string(),
        CrosstermKeyCode::Tab => "Tab".to_string(),
        CrosstermKeyCode::Esc => "Esc".to_string(),
        CrosstermKeyCode::Up => "Up".to_string(),
        CrosstermKeyCode::Down => "Down".to_string(),
        CrosstermKeyCode::Left => "Left".to_string(),
        CrosstermKeyCode::Right => "Right".to_string(),
        CrosstermKeyCode::F(n) => format!("F{n}"),
        _ => "Unknown".to_string(),
    };

    parts.push(key);
    parts.join("-")
}

fn log_error(err: std::io::Error) -> AltreError {
    AltreError::Application(format!("デバッグログの初期化に失敗しました: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::BackendOptions;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn inserts_character_via_controller() {
        let mut controller = BackendController::new(BackendOptions::default()).unwrap();
        let events = [KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)];
        let snapshot = controller.handle_key_events(&events).unwrap();
        assert_eq!(snapshot.buffer.lines.join("\n"), "a");
        assert_eq!(snapshot.buffer.cursor.column, 1);
    }

    #[test]
    fn generates_snapshot_without_input() {
        let mut controller = BackendController::new(BackendOptions::default()).unwrap();
        let snapshot = controller.snapshot().unwrap();
        // 初期バッファは scratch
        assert!(!snapshot.status.label.is_empty());
    }
}
