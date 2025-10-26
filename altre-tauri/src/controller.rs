use crate::keymap::KeySequencePayload;
use crate::logging::DebugLogger;
use crate::options::BackendOptions;
use crate::snapshot::EditorSnapshot;
use altre::error::{AltreError, Result};
use altre::ui::viewport::ViewportState;
use altre::Backend;
use crossterm::event::{KeyCode as CrosstermKeyCode, KeyEvent, KeyModifiers as CrosstermModifiers};
use serde::Serialize;
use serde_json::json;
use std::path::Path;

/// GUI から Rust バックエンドを操作するコントローラー
pub struct BackendController {
    backend: Backend,
    logger: Option<DebugLogger>,
    viewport_height: usize,
    viewport_width: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveResponse {
    pub success: bool,
    pub message: Option<String>,
    pub snapshot: EditorSnapshot,
}

/// `BackendController` は内部で `Rc` を使用するため `Send` を実装しないが、
/// ミューテックス越しに逐次アクセスする運用前提のため明示的に `Send` を許可する。
/// Tauri コマンドは `Mutex<BackendController>` 越しに利用し、並列アクセスは行わない。
unsafe impl Send for BackendController {}

impl BackendController {
    pub fn new(options: BackendOptions) -> Result<Self> {
        if let Some(dir) = options.working_directory.as_ref() {
            change_working_directory(dir)?;
        }

        let backend = Backend::new()?;
        let logger = match options.resolve_log_path() {
            Some(path) => Some(DebugLogger::new(path).map_err(log_error)?),
            None => None,
        };
        let mut controller = Self {
            backend,
            logger,
            viewport_height: 40,
            viewport_width: 120,
        };

        {
            let view = controller.backend.render_view();
            if let Some(viewport) = view.window_manager.focused_viewport_mut() {
                viewport.update_dimensions(controller.viewport_height, controller.viewport_width);
            }
        }

        if let Some(path) = options.initial_file.as_ref() {
            controller.open_initial_file(path)?;
        }

        Ok(controller)
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
        if events.is_empty() {
            return self.snapshot();
        }
        self.handle_key_events(&events)
    }

    pub fn open_file(&mut self, path: &str) -> Result<EditorSnapshot> {
        self.backend.open_file(path)?;
        self.log_event("open_file", &json!({ "path": path }))?;
        self.snapshot()
    }

    pub fn save_active_buffer(&mut self) -> Result<SaveResponse> {
        let events = [
            KeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermModifiers::CONTROL),
            KeyEvent::new(CrosstermKeyCode::Char('s'), CrosstermModifiers::CONTROL),
        ];
        let snapshot = self.handle_key_events(&events)?;
        let mode = snapshot.minibuffer.mode.clone();
        let message = snapshot.minibuffer.message.clone();
        let success = !matches!(mode.as_str(), "error" | "write-file" | "save-confirmation");

        self.log_event(
            "save_buffer",
            &json!({
                "success": success,
                "mode": mode,
                "message": message,
            }),
        )?;

        Ok(SaveResponse {
            success,
            message,
            snapshot,
        })
    }

    pub fn shutdown(&mut self) {
        self.backend.shutdown();
    }

    pub fn is_running(&self) -> bool {
        self.backend.is_running()
    }

    fn create_snapshot(&mut self) -> Result<EditorSnapshot> {
        let metadata = self.backend.render_metadata();
        let view = self.backend.render_view();
        if let Some(viewport) = view.window_manager.focused_viewport_mut() {
            viewport.update_dimensions(self.viewport_height, self.viewport_width);
        }
        let text = view.editor.to_string();
        let cursor = *view.editor.cursor();
        let viewport_state = view
            .window_manager
            .focused_viewport()
            .cloned()
            .unwrap_or_else(ViewportState::new);
        let snapshot =
            EditorSnapshot::new(&text, &cursor, &metadata, view.minibuffer, viewport_state);
        Ok(snapshot)
    }

    fn log_event<T: Serialize>(&self, tag: &str, payload: &T) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger.log_event(tag, payload).map_err(|err| {
                AltreError::Application(format!("デバッグログ出力に失敗しました: {err}"))
            })?;
        }
        Ok(())
    }

    fn open_initial_file(&mut self, path: &Path) -> Result<()> {
        let display = path.to_string_lossy().to_string();
        self.backend.open_file(&display)?;
        self.log_event("init_open_file", &json!({ "path": display }))?;
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

fn change_working_directory(path: &Path) -> Result<()> {
    std::env::set_current_dir(path).map_err(|err| {
        AltreError::Application(format!(
            "ワーキングディレクトリの変更に失敗しました ({}): {err}",
            path.to_string_lossy()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::BackendOptions;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn inserts_character_via_controller() {
        let temp = tempdir().unwrap();
        let options = BackendOptions {
            debug_log_path: Some(temp.path().join("log.jsonl")),
            ..Default::default()
        };
        let mut controller = BackendController::new(options).unwrap();
        let events = [KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)];
        let snapshot = controller.handle_key_events(&events).unwrap();
        assert_eq!(snapshot.buffer.lines.join("\n"), "a");
        assert_eq!(snapshot.buffer.cursor.column, 1);
    }

    #[test]
    fn generates_snapshot_without_input() {
        let temp = tempdir().unwrap();
        let options = BackendOptions {
            debug_log_path: Some(temp.path().join("log.jsonl")),
            ..Default::default()
        };
        let mut controller = BackendController::new(options).unwrap();
        let snapshot = controller.snapshot().unwrap();
        // 初期バッファは scratch
        assert!(!snapshot.status.label.is_empty());
    }

    #[test]
    fn save_active_buffer_writes_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("sample.txt");
        let options = BackendOptions {
            debug_log_path: Some(temp.path().join("log.jsonl")),
            ..Default::default()
        };
        let mut controller = BackendController::new(options).unwrap();

        controller
            .open_file(file_path.to_str().unwrap())
            .expect("ファイルオープンに失敗しました");

        controller
            .handle_key_events(&[KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)])
            .expect("文字入力に失敗しました");

        let response = controller
            .save_active_buffer()
            .expect("保存処理に失敗しました");

        assert!(response.success);
        assert!(!response.snapshot.status.is_modified);

        let content = fs::read_to_string(file_path).expect("保存結果の読み込みに失敗しました");
        assert_eq!(content, "a");
    }
}
