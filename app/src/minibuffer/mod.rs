//! ミニバッファシステム
//!
//! Emacs風のコマンド入力インターフェース、ファイル操作、補完機能を提供

use std::time::{Duration, Instant};

pub mod completion;
pub mod history;
pub mod prompt;
pub mod system;
pub mod commands;
pub mod ui;

// 公開API（既存）
pub use completion::{CompletionEngine, PathCompletion, CommandCompletion};
pub use prompt::{PromptManager, PromptResult};

// 新しい公開API
pub use system::{MinibufferSystem, MinibufferConfig, SystemState, SystemEvent, SystemResponse, FileOperation};
pub use commands::{
    CommandProcessor, CommandResult, CommandDefinition, CommandContext,
    FileOperationType, BufferOperationType, SystemOperationType,
};
pub use ui::{MinibufferRenderer, MinibufferUIConfig, MinibufferLayout, MinibufferStyles};

// 新しい公開API
use crate::input::keybinding::{Key, KeyCode};

/// ミニバッファの動作モード
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferMode {
    /// 非アクティブ状態
    Inactive,
    /// ファイルパス入力
    FindFile,
    /// コマンド実行入力
    ExecuteCommand,
    /// 評価入力
    EvalExpression,
    /// ファイル保存入力
    WriteFile,
    /// 保存確認
    SaveConfirmation,
    /// エラーメッセージ表示
    ErrorDisplay { message: String, expires_at: Instant },
    /// 情報メッセージ表示
    InfoDisplay { message: String, expires_at: Instant },
}

/// ミニバッファの状態
#[derive(Debug, Clone)]
pub struct MinibufferState {
    /// 現在のモード
    pub mode: MinibufferMode,
    /// 入力中のテキスト
    pub input: String,
    /// カーソル位置（文字単位）
    pub cursor_pos: usize,
    /// 現在のプロンプト
    pub prompt: String,
    /// 補完候補
    pub completions: Vec<String>,
    /// 選択中の補完候補インデックス
    pub selected_completion: Option<usize>,
    /// 履歴（セッション内のみ）
    pub history: history::SessionHistory,
    /// 履歴ナビゲーション位置
    pub history_index: Option<usize>,
}

impl Default for MinibufferState {
    fn default() -> Self {
        Self {
            mode: MinibufferMode::Inactive,
            input: String::new(),
            cursor_pos: 0,
            prompt: String::new(),
            completions: Vec::new(),
            selected_completion: None,
            history: history::SessionHistory::new(),
            history_index: None,
        }
    }
}

/// ミニバッファの外観設定
#[derive(Debug, Clone)]
pub struct MinibufferStyle {
    /// プロンプト部分のスタイル
    pub prompt_color: String,
    /// 入力テキストのスタイル
    pub input_color: String,
    /// 補完候補のスタイル
    pub completion_color: String,
    /// 選択された候補のスタイル
    pub selected_completion_color: String,
    /// エラーメッセージのスタイル
    pub error_color: String,
    /// ボーダーのスタイル
    pub border_color: String,
}

impl Default for MinibufferStyle {
    fn default() -> Self {
        Self {
            prompt_color: "blue".to_string(),
            input_color: "white".to_string(),
            completion_color: "gray".to_string(),
            selected_completion_color: "black_on_blue".to_string(),
            error_color: "red".to_string(),
            border_color: "dark_gray".to_string(),
        }
    }
}

/// ミニバッファの入力イベント
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferEvent {
    /// 文字入力
    Input(char),
    /// Backspace
    Backspace,
    /// Delete
    Delete,
    /// カーソル移動
    MoveCursor(CursorDirection),
    /// Enter（確定）
    Submit,
    /// Tab（補完）
    Complete,
    /// キャンセル（C-g）
    Cancel,
    /// 履歴ナビゲーション
    HistoryPrevious,
    HistoryNext,
    /// 補完候補ナビゲーション
    CompletionNext,
    CompletionPrevious,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorDirection {
    Left,
    Right,
    Home,
    End,
}

/// 入力処理の結果
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferResult {
    /// 処理継続
    Continue,
    /// コマンド実行
    Execute(String),
    /// 式評価
    EvalExpression(String),
    /// 保存用ファイルパス
    SaveFileAs(String),
    /// キャンセル
    Cancel,
    /// 無効な操作
    Invalid,
}

/// ミニバッファ関連のエラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum MinibufferError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<std::io::Error> for MinibufferError {
    fn from(error: std::io::Error) -> Self {
        MinibufferError::IoError(error.to_string())
    }
}

impl MinibufferError {
    /// ユーザーフレンドリーなメッセージに変換
    pub fn user_message(&self) -> String {
        match self {
            MinibufferError::FileNotFound(path) => {
                format!("ファイルが見つかりません: {}", path)
            }
            MinibufferError::PermissionDenied(path) => {
                format!("アクセス権限がありません: {}", path)
            }
            MinibufferError::InvalidPath(path) => {
                format!("無効なパスです: {}", path)
            }
            MinibufferError::IoError(err) => {
                format!("ファイル操作エラー: {}", err)
            }
            MinibufferError::CommandNotFound(cmd) => {
                format!("コマンドが見つかりません: {}", cmd)
            }
            MinibufferError::InvalidInput(input) => {
                format!("無効な入力です: {}", input)
            }
        }
    }
}

/// コマンド実行者のトレイト
pub trait CommandExecutor {
    /// コマンドを実行
    fn execute_command(&mut self, command: &str, args: &[String]) -> std::result::Result<String, MinibufferError>;

    /// 利用可能なコマンド一覧を取得
    fn get_available_commands(&self) -> Vec<String>;
}

/// キーバインドシステムとの統合用
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferAction {
    FindFile,
    ExecuteCommand,
    EvalExpression,
    SaveFile,
}

/// 新しいミニバッファコントローラー
pub struct ModernMinibuffer {
    /// 現在の状態
    state: MinibufferState,
    /// 外観設定
    style: MinibufferStyle,
    /// 補完エンジン
    completion_engine: Box<dyn completion::CompletionEngine>,
    /// コマンド実行者
    command_executor: Option<Box<dyn CommandExecutor>>,
}

impl std::fmt::Debug for ModernMinibuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModernMinibuffer")
            .field("state", &self.state)
            .field("style", &self.style)
            .field("completion_engine", &"<CompletionEngine>")
            .field("command_executor", &self.command_executor.as_ref().map(|_| "<CommandExecutor>"))
            .finish()
    }
}

impl ModernMinibuffer {
    /// 新しいミニバッファを作成
    pub fn new() -> Self {
        Self {
            state: MinibufferState::default(),
            style: MinibufferStyle::default(),
            completion_engine: Box::new(completion::PathCompletion::new()),
            command_executor: None,
        }
    }

    /// ファイル検索を開始
    pub fn start_find_file(&mut self, initial_path: Option<&str>) {
        self.state.mode = MinibufferMode::FindFile;
        self.state.prompt = "Find file: ".to_string();

        // カレントディレクトリパスを初期入力として設定
        self.state.input = initial_path.unwrap_or("").to_string();
        self.state.cursor_pos = self.state.input.chars().count();

        self.update_completions();
    }

    /// コマンド実行を開始
    pub fn start_execute_command(&mut self) {
        self.state.mode = MinibufferMode::ExecuteCommand;
        self.state.prompt = "M-x ".to_string();
        self.state.input.clear();
        self.state.cursor_pos = 0;
        self.update_completions();
    }

    /// ファイル保存を開始
    pub fn start_write_file(&mut self, initial_path: Option<&str>) {
        self.state.mode = MinibufferMode::WriteFile;
        self.state.prompt = "Save file: ".to_string();
        self.state.input = initial_path.unwrap_or("").to_string();
        self.state.cursor_pos = self.state.input.chars().count();
        self.update_completions();
    }

    /// 式評価を開始
    pub fn start_eval_expression(&mut self) {
        self.state.mode = MinibufferMode::EvalExpression;
        self.state.prompt = "Eval: ".to_string();
        self.state.input.clear();
        self.state.cursor_pos = 0;
        self.update_completions();
    }

    /// エラーメッセージを表示
    pub fn show_error(&mut self, message: String) {
        let expires_at = Instant::now() + Duration::from_secs(5); // QA.mdの回答
        self.state.mode = MinibufferMode::ErrorDisplay { message, expires_at };
    }

    /// 情報メッセージを表示
    pub fn show_info(&mut self, message: String) {
        let expires_at = Instant::now() + Duration::from_secs(3);
        self.state.mode = MinibufferMode::InfoDisplay { message, expires_at };
    }

    /// キー入力を処理
    pub fn handle_key(&mut self, key: Key) -> MinibufferResult {
        // メッセージ表示中の自動消去チェック
        self.check_message_expiry();

        match self.state.mode {
            MinibufferMode::Inactive => MinibufferResult::Continue,
            MinibufferMode::ErrorDisplay { .. } | MinibufferMode::InfoDisplay { .. } => {
                // メッセージ表示中は任意のキーで消去
                self.deactivate();
                MinibufferResult::Continue
            }
            _ => self.handle_input_key(key),
        }
    }

    /// 非アクティブ化
    pub fn deactivate(&mut self) {
        self.state.mode = MinibufferMode::Inactive;
        self.state.input.clear();
        self.state.completions.clear();
        self.state.selected_completion = None;
        self.state.cursor_pos = 0;
        self.state.history_index = None;
    }

    /// 現在の状態を取得
    pub fn state(&self) -> &MinibufferState {
        &self.state
    }

    /// 外観設定を取得
    pub fn style(&self) -> &MinibufferStyle {
        &self.style
    }

    /// アクティブかどうかを判定
    pub fn is_active(&self) -> bool {
        !matches!(self.state.mode, MinibufferMode::Inactive)
    }

    /// キーバインドシステムからの呼び出し
    pub fn handle_action(&mut self, action: MinibufferAction) -> MinibufferResult {
        match action {
            MinibufferAction::FindFile => {
                self.start_find_file(None);
                MinibufferResult::Continue
            }
            MinibufferAction::ExecuteCommand => {
                self.start_execute_command();
                MinibufferResult::Continue
            }
            MinibufferAction::EvalExpression => {
                self.start_eval_expression();
                MinibufferResult::Continue
            }
            MinibufferAction::SaveFile => {
                self.start_write_file(None);
                MinibufferResult::Continue
            }
        }
    }

    // 内部メソッド
    fn handle_input_key(&mut self, key: Key) -> MinibufferResult {
        let event = self.key_to_event(key);

        match event {
            MinibufferEvent::Input(ch) => {
                self.insert_char(ch);
                self.update_completions();
                MinibufferResult::Continue
            }
            MinibufferEvent::Backspace => {
                self.delete_backward();
                self.update_completions();
                MinibufferResult::Continue
            }
            MinibufferEvent::Delete => {
                self.delete_forward();
                self.update_completions();
                MinibufferResult::Continue
            }
            MinibufferEvent::Submit => {
                self.submit()
            }
            MinibufferEvent::Cancel => {
                self.cancel()
            }
            MinibufferEvent::Complete => {
                self.handle_completion();
                MinibufferResult::Continue
            }
            MinibufferEvent::MoveCursor(direction) => {
                self.move_cursor(direction);
                MinibufferResult::Continue
            }
            MinibufferEvent::CompletionNext => {
                self.select_next_completion();
                MinibufferResult::Continue
            }
            MinibufferEvent::CompletionPrevious => {
                self.select_previous_completion();
                MinibufferResult::Continue
            }
            MinibufferEvent::HistoryPrevious => {
                self.history_previous();
                MinibufferResult::Continue
            }
            MinibufferEvent::HistoryNext => {
                self.history_next();
                MinibufferResult::Continue
            }
        }
    }

    fn key_to_event(&self, key: Key) -> MinibufferEvent {
        match key.code {
            KeyCode::Char(ch) if !key.modifiers.ctrl && !key.modifiers.alt => {
                MinibufferEvent::Input(ch)
            }
            KeyCode::Backspace => MinibufferEvent::Backspace,
            KeyCode::Delete => MinibufferEvent::Delete,
            KeyCode::Enter => MinibufferEvent::Submit,
            KeyCode::Tab => MinibufferEvent::Complete,
            KeyCode::Char('g') if key.modifiers.ctrl => MinibufferEvent::Cancel,
            KeyCode::Left => MinibufferEvent::MoveCursor(CursorDirection::Left),
            KeyCode::Right => MinibufferEvent::MoveCursor(CursorDirection::Right),
            KeyCode::Char('a') if key.modifiers.ctrl => MinibufferEvent::MoveCursor(CursorDirection::Home),
            KeyCode::Char('e') if key.modifiers.ctrl => MinibufferEvent::MoveCursor(CursorDirection::End),
            KeyCode::Down => MinibufferEvent::CompletionNext,
            KeyCode::Up => MinibufferEvent::CompletionPrevious,
            KeyCode::Char('p') if key.modifiers.ctrl => MinibufferEvent::HistoryPrevious,
            KeyCode::Char('n') if key.modifiers.ctrl => MinibufferEvent::HistoryNext,
            _ => MinibufferEvent::Input('\0'), // 無効な入力として扱う
        }
    }

    fn insert_char(&mut self, ch: char) {
        if ch == '\0' {
            return; // 無効な文字は無視
        }

        let byte_pos = self.cursor_byte_pos();
        self.state.input.insert(byte_pos, ch);
        self.state.cursor_pos += 1;
    }

    fn delete_backward(&mut self) {
        if self.state.cursor_pos == 0 {
            return;
        }

        let byte_pos = self.cursor_byte_pos();
        if byte_pos > 0 {
            // 前の文字の境界を見つける
            let mut char_start = byte_pos - 1;
            while char_start > 0 && !self.state.input.is_char_boundary(char_start) {
                char_start -= 1;
            }

            self.state.input.drain(char_start..byte_pos);
            self.state.cursor_pos -= 1;
        }
    }

    fn delete_forward(&mut self) {
        let byte_pos = self.cursor_byte_pos();
        if byte_pos >= self.state.input.len() {
            return;
        }

        // 次の文字の境界を見つける
        let mut char_end = byte_pos + 1;
        while char_end < self.state.input.len() && !self.state.input.is_char_boundary(char_end) {
            char_end += 1;
        }

        self.state.input.drain(byte_pos..char_end);
    }

    fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Left => {
                if self.state.cursor_pos > 0 {
                    self.state.cursor_pos -= 1;
                }
            }
            CursorDirection::Right => {
                let char_count = self.state.input.chars().count();
                if self.state.cursor_pos < char_count {
                    self.state.cursor_pos += 1;
                }
            }
            CursorDirection::Home => {
                self.state.cursor_pos = 0;
            }
            CursorDirection::End => {
                self.state.cursor_pos = self.state.input.chars().count();
            }
        }
    }

    fn cursor_byte_pos(&self) -> usize {
        self.state.input
            .char_indices()
            .nth(self.state.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.state.input.len())
    }

    fn update_completions(&mut self) {
        // 入力が短すぎる場合は補完しない（パフォーマンス考慮）
        if self.state.input.len() < 2 {
            self.state.completions.clear();
            self.state.selected_completion = None;
            return;
        }

        match self.state.mode {
            MinibufferMode::FindFile | MinibufferMode::WriteFile => {
                let completions = self.completion_engine.complete(&self.state.input);
                let mut limited_completions = completions.unwrap_or_default();
                limited_completions.truncate(50); // QA.mdの回答
                self.state.completions = limited_completions;
            }
            MinibufferMode::ExecuteCommand => {
                // コマンド補完は将来実装
                self.state.completions.clear();
            }
            _ => {
                self.state.completions.clear();
            }
        }

        self.state.selected_completion = if self.state.completions.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    fn handle_completion(&mut self) {
        if self.state.completions.is_empty() {
            return;
        }

        // 最初の候補を使用
        if let Some(completion) = self.state.completions.first() {
            self.state.input = completion.clone();
            self.state.cursor_pos = self.state.input.chars().count();
            self.update_completions();
        }
    }

    fn select_next_completion(&mut self) {
        if self.state.completions.is_empty() {
            return;
        }

        match self.state.selected_completion {
            Some(index) => {
                let next_index = (index + 1) % self.state.completions.len();
                self.state.selected_completion = Some(next_index);
            }
            None => {
                self.state.selected_completion = Some(0);
            }
        }
    }

    fn select_previous_completion(&mut self) {
        if self.state.completions.is_empty() {
            return;
        }

        match self.state.selected_completion {
            Some(index) => {
                let prev_index = if index == 0 {
                    self.state.completions.len() - 1
                } else {
                    index - 1
                };
                self.state.selected_completion = Some(prev_index);
            }
            None => {
                self.state.selected_completion = Some(self.state.completions.len() - 1);
            }
        }
    }

    fn history_previous(&mut self) {
        if self.state.history.is_empty() {
            return;
        }

        let next_index = match self.state.history_index {
            Some(index) => {
                if index + 1 < self.state.history.len() {
                    index + 1
                } else {
                    return;
                }
            }
            None => 0,
        };

        if let Some(entry) = self.state.history.get_entry(next_index) {
            self.state.input = entry.clone();
            self.state.cursor_pos = self.state.input.chars().count();
            self.state.history_index = Some(next_index);
            self.update_completions();
        }
    }

    fn history_next(&mut self) {
        match self.state.history_index {
            Some(index) => {
                if index == 0 {
                    self.state.input.clear();
                    self.state.cursor_pos = 0;
                    self.state.history_index = None;
                } else {
                    let next_index = index - 1;
                    if let Some(entry) = self.state.history.get_entry(next_index) {
                        self.state.input = entry.clone();
                        self.state.cursor_pos = self.state.input.chars().count();
                        self.state.history_index = Some(next_index);
                    }
                }
                self.update_completions();
            }
            None => {
                // 履歴ナビゲーション中でない場合は何もしない
            }
        }
    }

    fn submit(&mut self) -> MinibufferResult {
        let input = self.state.input.clone();

        match &self.state.mode {
            MinibufferMode::FindFile => {
                if input.is_empty() {
                    self.show_error("No file specified".to_string());
                    MinibufferResult::Continue
                } else {
                    // 履歴に追加
                    self.add_to_history(input.clone());
                    self.deactivate();
                    MinibufferResult::Execute(format!("find-file {}", input))
                }
            }
            MinibufferMode::ExecuteCommand => {
                if input.is_empty() {
                    self.show_error("No command specified".to_string());
                    MinibufferResult::Continue
                } else {
                    self.add_to_history(input.clone());
                    self.deactivate();
                    MinibufferResult::Execute(input)
                }
            }
            MinibufferMode::EvalExpression => {
                if input.is_empty() {
                    self.show_error("式が入力されていません".to_string());
                    MinibufferResult::Continue
                } else {
                    self.add_to_history(input.clone());
                    self.deactivate();
                    MinibufferResult::EvalExpression(input)
                }
            }
            MinibufferMode::WriteFile => {
                if input.is_empty() {
                    self.show_error("ファイル名を入力してください".to_string());
                    MinibufferResult::Continue
                } else {
                    self.add_to_history(input.clone());
                    self.deactivate();
                    MinibufferResult::SaveFileAs(input)
                }
            }
            _ => MinibufferResult::Continue,
        }
    }

    fn cancel(&mut self) -> MinibufferResult {
        self.deactivate();
        MinibufferResult::Cancel
    }

    fn add_to_history(&mut self, entry: String) {
        if !entry.is_empty() {
            self.state.history.add_entry(entry);
        }
    }

    fn check_message_expiry(&mut self) {
        let now = Instant::now();

        match &self.state.mode {
            MinibufferMode::ErrorDisplay { expires_at, .. } |
            MinibufferMode::InfoDisplay { expires_at, .. } => {
                if now >= *expires_at {
                    self.deactivate();
                }
            }
            _ => {}
        }
    }
}

impl Default for ModernMinibuffer {
    fn default() -> Self {
        Self::new()
    }
}
