//! イベントハンドリング
//!
//! キーボード入力やターミナルイベントの処理

use crate::error::Result;
use super::{CommandProcessor, CommandResult};
use super::keybinding::{KeyProcessResult, ModernKeyMap};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// 入力ハンドラー
pub struct InputHandler {
    /// キーマップ
    keymap: ModernKeyMap,
    /// コマンド処理器
    command_processor: CommandProcessor,
    /// 入力タイムアウト（ミリ秒）
    timeout: Duration,
}

impl InputHandler {
    /// 新しい入力ハンドラーを作成
    pub fn new() -> Self {
        Self {
            keymap: ModernKeyMap::new(),
            command_processor: CommandProcessor::new(),
            timeout: Duration::from_millis(100),
        }
    }

    /// タイムアウト付きで入力ハンドラーを作成
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            keymap: ModernKeyMap::new(),
            command_processor: CommandProcessor::new(),
            timeout,
        }
    }

    /// 入力イベントを処理
    pub fn handle_input(&mut self) -> Result<InputResult> {
        // イベントの可用性をチェック
        if !event::poll(self.timeout)? {
            return Ok(InputResult::Timeout);
        }

        // イベントを読み取り
        match event::read()? {
            Event::Key(key_event) => self.handle_key_event(key_event),
            Event::Resize(cols, rows) => Ok(InputResult::Resize { cols, rows }),
            Event::Mouse(_) => Ok(InputResult::Ignored), // マウスは未サポート
            Event::FocusGained => Ok(InputResult::Ignored),
            Event::FocusLost => Ok(InputResult::Ignored),
            Event::Paste(_) => Ok(InputResult::Ignored), // ペーストは未サポート
        }
    }

    /// キーイベントを処理
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<InputResult> {
        // 特殊キーの処理
        if self.handle_special_keys(&key_event) {
            return Ok(InputResult::Handled);
        }

        match self.keymap.process_key_event(key_event) {
            KeyProcessResult::Action(action) => {
                if let Some(command) = action.to_command() {
                    let result = self.command_processor.execute(command);
                    Ok(InputResult::Command { result })
                } else {
                    Ok(InputResult::Ignored)
                }
            }
            KeyProcessResult::PartialMatch => Ok(InputResult::Prefix),
            KeyProcessResult::NoMatch => {
                self.keymap.reset_partial_match();
                Ok(InputResult::Unbound {
                    key: format!("{:?}", key_event),
                })
            }
        }
    }

    /// 特殊キーの処理（キーマップを迂回）
    fn handle_special_keys(&mut self, key_event: &KeyEvent) -> bool {
        match (key_event.code, key_event.modifiers) {
            // C-g: キーシーケンスのキャンセル
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.keymap.reset_partial_match();
                true
            }
            // ESC: キーシーケンスのキャンセル
            (KeyCode::Esc, _) => {
                self.keymap.reset_partial_match();
                true
            }
            _ => false,
        }
    }

    /// 現在のキーシーケンス状態を取得
    pub fn current_key_sequence(&self) -> String {
        self
            .keymap
            .current_prefix_label()
            .unwrap_or("")
            .to_string()
    }

    /// キーマップをリセット
    pub fn reset_keymap(&mut self) {
        self.keymap.reset_partial_match();
    }

    /// 文字入力かどうかを判定
    pub fn is_character_input(&self, key_event: &KeyEvent) -> bool {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char(_), KeyModifiers::NONE) => true,
            (KeyCode::Char(_), KeyModifiers::SHIFT) => true,
            (KeyCode::Enter, _) => true,
            (KeyCode::Tab, _) => true,
            _ => false,
        }
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 入力処理の結果
#[derive(Debug)]
pub enum InputResult {
    /// コマンドが実行された
    Command { result: CommandResult },
    /// プレフィックスキーが入力された
    Prefix,
    /// 未バインドキーが入力された
    Unbound { key: String },
    /// タイムアウト
    Timeout,
    /// ターミナルサイズ変更
    Resize { cols: u16, rows: u16 },
    /// 処理済み（特殊キー等）
    Handled,
    /// 無視（マウス等）
    Ignored,
}

impl InputResult {
    /// コマンド実行結果を持つかチェック
    pub fn has_command_result(&self) -> bool {
        matches!(self, InputResult::Command { .. })
    }

    /// コマンド実行結果を取得
    pub fn command_result(&self) -> Option<&CommandResult> {
        match self {
            InputResult::Command { result } => Some(result),
            _ => None,
        }
    }

    /// プレフィックス状態かチェック
    pub fn is_prefix(&self) -> bool {
        matches!(self, InputResult::Prefix)
    }

    /// 未バインドキーかチェック
    pub fn is_unbound(&self) -> bool {
        matches!(self, InputResult::Unbound { .. })
    }
}

/// イベント処理器（より高レベルなインターフェース）
pub struct EventProcessor {
    input_handler: InputHandler,
    last_error: Option<String>,
}

impl EventProcessor {
    /// 新しいイベント処理器を作成
    pub fn new() -> Self {
        Self {
            input_handler: InputHandler::new(),
            last_error: None,
        }
    }

    /// イベント処理のメインループ
    pub fn process_events(&mut self) -> Result<ProcessResult> {
        match self.input_handler.handle_input() {
            Ok(input_result) => {
                self.last_error = None;
                Ok(self.convert_input_result(input_result))
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.last_error = Some(error_msg.clone());
                Ok(ProcessResult::Error { message: error_msg })
            }
        }
    }

    /// InputResultをProcessResultに変換
    fn convert_input_result(&mut self, input_result: InputResult) -> ProcessResult {
        match input_result {
            InputResult::Command { result } => {
                if result.should_quit {
                    ProcessResult::Quit
                } else if result.success {
                    ProcessResult::CommandExecuted {
                        message: result.message,
                        needs_refresh: result.needs_refresh,
                    }
                } else {
                    ProcessResult::Error {
                        message: result.message.unwrap_or_else(|| "コマンドエラー".to_string())
                    }
                }
            }
            InputResult::Prefix => ProcessResult::PrefixKey,
            InputResult::Unbound { key } => ProcessResult::UnboundKey { key },
            InputResult::Timeout => ProcessResult::Timeout,
            InputResult::Resize { cols, rows } => ProcessResult::Resize { cols, rows },
            InputResult::Handled | InputResult::Ignored => ProcessResult::Continue,
        }
    }

    /// 最後のエラーを取得
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// キーシーケンス状態を取得
    pub fn key_sequence_status(&self) -> String {
        self.input_handler.current_key_sequence()
    }
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// 処理結果
#[derive(Debug)]
pub enum ProcessResult {
    /// コマンドが実行された
    CommandExecuted {
        message: Option<String>,
        needs_refresh: bool,
    },
    /// プレフィックスキーが入力された
    PrefixKey,
    /// 未バインドキーが入力された
    UnboundKey { key: String },
    /// エラーが発生
    Error { message: String },
    /// タイムアウト
    Timeout,
    /// ターミナルサイズ変更
    Resize { cols: u16, rows: u16 },
    /// 継続（何もしない）
    Continue,
    /// アプリケーション終了
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new();
        assert_eq!(handler.timeout, Duration::from_millis(100));
    }

    #[test]
    fn test_character_input_detection() {
        let handler = InputHandler::new();

        let char_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(handler.is_character_input(&char_event));

        let ctrl_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(!handler.is_character_input(&ctrl_event));

        let enter_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(handler.is_character_input(&enter_event));
    }

    #[test]
    fn test_special_keys() {
        let mut handler = InputHandler::new();

        let ctrl_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        assert!(handler.handle_special_keys(&ctrl_g));

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(handler.handle_special_keys(&esc));

        let normal_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(!handler.handle_special_keys(&normal_key));
    }

    #[test]
    fn test_input_result_methods() {
        let cmd_result = InputResult::Command {
            result: CommandResult::success()
        };
        assert!(cmd_result.has_command_result());

        let prefix_result = InputResult::Prefix;
        assert!(prefix_result.is_prefix());

        let unbound_result = InputResult::Unbound {
            key: "test".to_string()
        };
        assert!(unbound_result.is_unbound());
    }
}
