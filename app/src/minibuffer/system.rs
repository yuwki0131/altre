//! ミニバッファシステムのメイン実装
//!
//! Emacsスタイルのミニバッファ機能の統合システム

use crate::error::Result;
use crate::alisp::Interpreter;
use crate::alisp::integration::eval_in_minibuffer;
use crate::input::keybinding::Key;
use super::{
    ModernMinibuffer, MinibufferResult, MinibufferAction,
    completion::{PathCompletion, CommandCompletion},
};
use std::time::{Duration, Instant};

/// ミニバッファシステムのメイン実装
pub struct MinibufferSystem {
    /// 現在のミニバッファ
    minibuffer: ModernMinibuffer,
    /// パス補完エンジン
    path_completion: PathCompletion,
    /// コマンド補完エンジン
    command_completion: CommandCompletion,
    /// 最後の更新時刻
    last_update: Instant,
    /// システム設定
    config: MinibufferConfig,
    /// alispインタプリタ
    alisp_interpreter: Interpreter,
}

/// ミニバッファシステムの設定
#[derive(Debug, Clone)]
pub struct MinibufferConfig {
    /// 自動補完の有効化
    pub auto_completion: bool,
    /// 補完候補の最大数
    pub max_completions: usize,
    /// エラーメッセージの表示時間
    pub error_display_duration: Duration,
    /// 情報メッセージの表示時間
    pub info_display_duration: Duration,
    /// 履歴の最大サイズ
    pub max_history_size: usize,
    /// 隠しファイルの表示
    pub show_hidden_files: bool,
}

impl Default for MinibufferConfig {
    fn default() -> Self {
        Self {
            auto_completion: true,
            max_completions: 50,
            error_display_duration: Duration::from_secs(5),
            info_display_duration: Duration::from_secs(3),
            max_history_size: 100,
            show_hidden_files: false,
        }
    }
}

/// ミニバッファシステムの状態
#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    /// 非アクティブ
    Inactive,
    /// ファイル検索モード
    FindFile,
    /// コマンド実行モード
    ExecuteCommand,
    /// エラー表示モード
    ErrorDisplay,
    /// 情報表示モード
    InfoDisplay,
}

/// システムイベント
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// キー入力
    KeyInput(Key),
    /// アクション要求
    Action(MinibufferAction),
    /// エラー表示要求
    ShowError(String),
    /// 情報表示要求
    ShowInfo(String),
    /// 定期更新
    Update,
}

/// システムの応答
#[derive(Debug, Clone)]
pub enum SystemResponse {
    /// 処理継続
    Continue,
    /// コマンド実行要求
    ExecuteCommand(String),
    /// ファイル操作要求
    FileOperation(FileOperation),
    /// バッファ切り替え要求
    SwitchBuffer(String),
    /// バッファ削除要求
    KillBuffer(String),
    /// バッファ一覧表示
    ListBuffers,
    /// クエリ置換開始
    QueryReplace { pattern: String, replacement: String, is_regex: bool },
    /// システム終了要求
    Quit,
    /// 何もしない
    None,
}

/// ファイル操作の種類
#[derive(Debug, Clone)]
pub enum FileOperation {
    /// ファイルを開く
    Open(String),
    /// ファイルを保存
    Save,
    /// 名前を付けて保存
    SaveAs(String),
}

impl MinibufferSystem {
    /// 新しいミニバッファシステムを作成
    pub fn new() -> Self {
        Self::with_config(MinibufferConfig::default())
    }

    /// 設定付きでミニバッファシステムを作成
    pub fn with_config(config: MinibufferConfig) -> Self {
        let path_completion = PathCompletion::new()
            .with_hidden_files(config.show_hidden_files);

        Self {
            minibuffer: ModernMinibuffer::new(),
            path_completion,
            command_completion: CommandCompletion::new(),
            last_update: Instant::now(),
            config,
            alisp_interpreter: Interpreter::new(),
        }
    }

    /// システムの状態を取得
    pub fn state(&self) -> SystemState {
        match &self.minibuffer.state().mode {
            super::MinibufferMode::Inactive => SystemState::Inactive,
            super::MinibufferMode::FindFile => SystemState::FindFile,
            super::MinibufferMode::ExecuteCommand => SystemState::ExecuteCommand,
            super::MinibufferMode::EvalExpression => SystemState::ExecuteCommand,
            super::MinibufferMode::SwitchBuffer | super::MinibufferMode::KillBuffer => SystemState::ExecuteCommand,
            super::MinibufferMode::ErrorDisplay { .. } => SystemState::ErrorDisplay,
            super::MinibufferMode::InfoDisplay { .. } => SystemState::InfoDisplay,
            _ => SystemState::Inactive,
        }
    }

    /// ミニバッファがアクティブかどうか
    pub fn is_active(&self) -> bool {
        self.minibuffer.is_active()
    }

    /// メッセージ表示中かどうか
    pub fn is_message_displayed(&self) -> bool {
        matches!(
            self.minibuffer.state().mode,
            super::MinibufferMode::ErrorDisplay { .. } | super::MinibufferMode::InfoDisplay { .. }
        )
    }

    /// 現在の入力内容を取得
    pub fn current_input(&self) -> &str {
        &self.minibuffer.state().input
    }

    /// 現在のプロンプトを取得
    pub fn current_prompt(&self) -> &str {
        &self.minibuffer.state().prompt
    }

    /// 補完候補を取得
    pub fn completions(&self) -> &[String] {
        &self.minibuffer.state().completions
    }

    /// 選択中の補完候補インデックスを取得
    pub fn selected_completion(&self) -> Option<usize> {
        self.minibuffer.state().selected_completion
    }

    /// 内部のミニバッファ状態を取得
    pub fn minibuffer_state(&self) -> &super::MinibufferState {
        self.minibuffer.state()
    }

    /// システムイベントを処理
    pub fn handle_event(&mut self, event: SystemEvent) -> Result<SystemResponse> {
        match event {
            SystemEvent::KeyInput(key) => self.handle_key_input(key),
            SystemEvent::Action(action) => self.handle_action(action),
            SystemEvent::ShowError(message) => {
                self.minibuffer.show_error(message);
                Ok(SystemResponse::Continue)
            }
            SystemEvent::ShowInfo(message) => {
                self.minibuffer.show_info(message);
                Ok(SystemResponse::Continue)
            }
            SystemEvent::Update => self.handle_update(),
        }
    }

    /// キー入力を処理
    fn handle_key_input(&mut self, key: Key) -> Result<SystemResponse> {
        let result = self.minibuffer.handle_key(key);

        match result {
            MinibufferResult::Continue => Ok(SystemResponse::Continue),
            MinibufferResult::Execute(command) => self.handle_execute_result(command),
            MinibufferResult::SwitchBuffer(name) => Ok(SystemResponse::SwitchBuffer(name)),
            MinibufferResult::KillBuffer(name) => Ok(SystemResponse::KillBuffer(name)),
            MinibufferResult::EvalExpression(expr) => self.handle_eval_expression(expr),
            MinibufferResult::SaveFileAs(path) => Ok(SystemResponse::FileOperation(FileOperation::SaveAs(path))),
            MinibufferResult::QueryReplace { pattern, replacement, is_regex } => Ok(SystemResponse::QueryReplace { pattern, replacement, is_regex }),
            MinibufferResult::Cancel => {
                self.minibuffer.deactivate();
                Ok(SystemResponse::Continue)
            }
            MinibufferResult::Invalid => Ok(SystemResponse::None),
        }
    }

    /// アクションを処理
    fn handle_action(&mut self, action: MinibufferAction) -> Result<SystemResponse> {
        let result = self.minibuffer.handle_action(action);

        match result {
            MinibufferResult::Continue => Ok(SystemResponse::Continue),
            MinibufferResult::Execute(command) => self.handle_execute_result(command),
            MinibufferResult::SwitchBuffer(name) => Ok(SystemResponse::SwitchBuffer(name)),
            MinibufferResult::KillBuffer(name) => Ok(SystemResponse::KillBuffer(name)),
            MinibufferResult::EvalExpression(expr) => self.handle_eval_expression(expr),
            MinibufferResult::SaveFileAs(path) => Ok(SystemResponse::FileOperation(FileOperation::SaveAs(path))),
            MinibufferResult::QueryReplace { pattern, replacement, is_regex } => Ok(SystemResponse::QueryReplace { pattern, replacement, is_regex }),
            MinibufferResult::Cancel => Ok(SystemResponse::Continue),
            MinibufferResult::Invalid => Ok(SystemResponse::None),
        }
    }

    /// 実行結果を処理
    fn handle_execute_result(&mut self, command: String) -> Result<SystemResponse> {
        if command.starts_with("find-file ") {
            let filename = command.strip_prefix("find-file ").unwrap_or("");
            if filename.is_empty() {
                self.minibuffer.show_error("No file specified".to_string());
                Ok(SystemResponse::Continue)
            } else {
                Ok(SystemResponse::FileOperation(FileOperation::Open(filename.to_string())))
            }
        } else if command == "save-buffer" {
            Ok(SystemResponse::FileOperation(FileOperation::Save))
        } else if command == "save-some-buffers" {
            Ok(SystemResponse::FileOperation(FileOperation::Save))
        } else if command == "write-file" || command == "save-buffer-as" {
            self.minibuffer.start_write_file(None);
            Ok(SystemResponse::Continue)
        } else if let Some(buffer_name) = command.strip_prefix("switch-to-buffer ") {
            Ok(SystemResponse::SwitchBuffer(buffer_name.trim().to_string()))
        } else if command == "switch-to-buffer" {
            Ok(SystemResponse::SwitchBuffer(String::new()))
        } else if let Some(buffer_name) = command.strip_prefix("kill-buffer ") {
            Ok(SystemResponse::KillBuffer(buffer_name.trim().to_string()))
        } else if command == "kill-buffer" {
            Ok(SystemResponse::KillBuffer(String::new()))
        } else if command == "list-buffers" {
            Ok(SystemResponse::ListBuffers)
        } else if command == "query-replace" {
            self.minibuffer.start_query_replace(false, None);
            Ok(SystemResponse::Continue)
        } else if command == "query-replace-regexp" {
            self.minibuffer.start_query_replace(true, None);
            Ok(SystemResponse::Continue)
        } else if let Some(expr) = command.strip_prefix("eval-expression ") {
            self.handle_eval_expression(expr.to_string())
        } else if command == "eval-expression" {
            self.minibuffer.start_eval_expression();
            Ok(SystemResponse::Continue)
        } else if let Some(path) = command.strip_prefix("write-file ") {
            let trimmed = path.trim();
            if trimmed.is_empty() {
                self.minibuffer.show_error("ファイル名を入力してください".to_string());
                Ok(SystemResponse::Continue)
            } else {
                Ok(SystemResponse::FileOperation(FileOperation::SaveAs(trimmed.to_string())))
            }
        } else if command == "quit" || command == "save-buffers-kill-terminal" {
            Ok(SystemResponse::Quit)
        } else {
            // その他のコマンドは直接実行
            Ok(SystemResponse::ExecuteCommand(command))
        }
    }

    fn handle_eval_expression(&mut self, expr: String) -> Result<SystemResponse> {
        let expression = expr.trim();

        if expression.is_empty() {
            self.minibuffer.show_error("式が入力されていません".to_string());
            return Ok(SystemResponse::Continue);
        }

        let outcome = eval_in_minibuffer(&mut self.alisp_interpreter, expression);

        if outcome.is_error {
            self.minibuffer.show_error(outcome.output);
        } else {
            let mut message = outcome.output;
            if !outcome.messages.is_empty() {
                let extras = outcome.messages.join(" | ");
                if !extras.is_empty() {
                    message = format!("{} ({})", message, extras);
                }
            }
            self.minibuffer.show_info(message);
        }

        Ok(SystemResponse::Continue)
    }

    /// 定期更新を処理
    fn handle_update(&mut self) -> Result<SystemResponse> {
        self.last_update = Instant::now();

        // メッセージの自動消去などの処理
        // (ModernMinibufferが内部で処理)

        Ok(SystemResponse::Continue)
    }

    /// ファイル検索を開始
    pub fn start_find_file(&mut self, initial_path: Option<&str>) -> Result<SystemResponse> {
        self.minibuffer.start_find_file(initial_path);
        Ok(SystemResponse::Continue)
    }

    /// コマンド実行を開始
    pub fn start_execute_command(&mut self) -> Result<SystemResponse> {
        self.minibuffer.start_execute_command();
        Ok(SystemResponse::Continue)
    }

    /// ファイル保存を開始
    pub fn start_write_file(&mut self, initial_path: Option<&str>) -> Result<SystemResponse> {
        self.minibuffer.start_write_file(initial_path);
        Ok(SystemResponse::Continue)
    }

    /// バッファ切り替えを開始
    pub fn start_switch_buffer(&mut self, buffers: &[String], initial: Option<&str>) -> Result<SystemResponse> {
        self.minibuffer.start_switch_buffer(buffers, initial);
        Ok(SystemResponse::Continue)
    }

    /// バッファ削除を開始
    pub fn start_kill_buffer(&mut self, buffers: &[String], initial: Option<&str>) -> Result<SystemResponse> {
        self.minibuffer.start_kill_buffer(buffers, initial);
        Ok(SystemResponse::Continue)
    }

    /// 式評価を開始
    pub fn start_eval_expression(&mut self) -> Result<SystemResponse> {
        self.minibuffer.start_eval_expression();
        Ok(SystemResponse::Continue)
    }

    /// エラーメッセージを表示
    pub fn show_error(&mut self, message: impl Into<String>) -> Result<SystemResponse> {
        self.minibuffer.show_error(message.into());
        Ok(SystemResponse::Continue)
    }

    /// 情報メッセージを表示
    pub fn show_info(&mut self, message: impl Into<String>) -> Result<SystemResponse> {
        self.minibuffer.show_info(message.into());
        Ok(SystemResponse::Continue)
    }

    /// 情報メッセージを任意の時間表示
    pub fn show_info_with_duration(
        &mut self,
        message: impl Into<String>,
        duration: Option<Duration>,
    ) -> Result<SystemResponse> {
        self.minibuffer
            .show_info_with_duration(message.into(), duration);
        Ok(SystemResponse::Continue)
    }

    /// ステータスメッセージを設定
    pub fn set_status_message(&mut self, message: Option<String>) {
        self.minibuffer.set_status_message(message);
    }

    /// ミニバッファを非アクティブ化
    pub fn deactivate(&mut self) {
        self.minibuffer.deactivate();
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: MinibufferConfig) {
        self.config = config;

        // パス補完の設定を更新
        self.path_completion = PathCompletion::new()
            .with_hidden_files(self.config.show_hidden_files);
    }

    /// 現在の設定を取得
    pub fn config(&self) -> &MinibufferConfig {
        &self.config
    }

    /// コマンドを補完エンジンに追加
    pub fn add_command(&mut self, command: String) {
        self.command_completion.add_command(command);
    }

    /// 利用可能なコマンド一覧を取得
    pub fn available_commands(&self) -> &[String] {
        self.command_completion.all_commands()
    }

    /// パフォーマンス統計を取得
    pub fn performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            last_update: self.last_update,
            uptime: self.last_update.elapsed(),
            completions_count: self.minibuffer.state().completions.len(),
            history_size: self.minibuffer.state().history.len(),
        }
    }
}

/// パフォーマンス統計情報
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// 最後の更新時刻
    pub last_update: Instant,
    /// システム稼働時間
    pub uptime: Duration,
    /// 現在の補完候補数
    pub completions_count: usize,
    /// 履歴サイズ
    pub history_size: usize,
}

impl Default for MinibufferSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// ミニバッファシステムのファクトリ
pub struct MinibufferSystemBuilder {
    config: MinibufferConfig,
}

impl MinibufferSystemBuilder {
    /// 新しいビルダーを作成
    pub fn new() -> Self {
        Self {
            config: MinibufferConfig::default(),
        }
    }

    /// 自動補完を設定
    pub fn auto_completion(mut self, enabled: bool) -> Self {
        self.config.auto_completion = enabled;
        self
    }

    /// 最大補完候補数を設定
    pub fn max_completions(mut self, max: usize) -> Self {
        self.config.max_completions = max;
        self
    }

    /// エラー表示時間を設定
    pub fn error_display_duration(mut self, duration: Duration) -> Self {
        self.config.error_display_duration = duration;
        self
    }

    /// 情報表示時間を設定
    pub fn info_display_duration(mut self, duration: Duration) -> Self {
        self.config.info_display_duration = duration;
        self
    }

    /// 最大履歴サイズを設定
    pub fn max_history_size(mut self, size: usize) -> Self {
        self.config.max_history_size = size;
        self
    }

    /// 隠しファイル表示を設定
    pub fn show_hidden_files(mut self, show: bool) -> Self {
        self.config.show_hidden_files = show;
        self
    }

    /// システムを構築
    pub fn build(self) -> MinibufferSystem {
        MinibufferSystem::with_config(self.config)
    }
}

impl Default for MinibufferSystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minibuffer_system_creation() {
        let system = MinibufferSystem::new();
        assert_eq!(system.state(), SystemState::Inactive);
        assert!(!system.is_active());
    }

    #[test]
    fn test_system_builder() {
        let system = MinibufferSystemBuilder::new()
            .auto_completion(false)
            .max_completions(25)
            .show_hidden_files(true)
            .build();

        assert!(!system.config().auto_completion);
        assert_eq!(system.config().max_completions, 25);
        assert!(system.config().show_hidden_files);
    }

    #[test]
    fn test_find_file_action() {
        let mut system = MinibufferSystem::new();

        let response = system.handle_action(MinibufferAction::FindFile).unwrap();
        assert!(matches!(response, SystemResponse::Continue));
        assert_eq!(system.state(), SystemState::FindFile);
        assert!(system.is_active());
    }

    #[test]
    fn test_execute_command_action() {
        let mut system = MinibufferSystem::new();

        let response = system.handle_action(MinibufferAction::ExecuteCommand).unwrap();
        assert!(matches!(response, SystemResponse::Continue));
        assert_eq!(system.state(), SystemState::ExecuteCommand);
    }

    #[test]
    fn test_eval_expression_action() {
        let mut system = MinibufferSystem::new();

        let response = system.handle_action(MinibufferAction::EvalExpression).unwrap();
        assert!(matches!(response, SystemResponse::Continue));
        assert_eq!(system.state(), SystemState::ExecuteCommand);
        assert!(system.is_active());
    }

    #[test]
    fn test_error_display() {
        let mut system = MinibufferSystem::new();

        let response = system.show_error("Test error").unwrap();
        assert!(matches!(response, SystemResponse::Continue));
        assert_eq!(system.state(), SystemState::ErrorDisplay);
    }

    #[test]
    fn test_info_display() {
        let mut system = MinibufferSystem::new();

        let response = system.show_info("Test info").unwrap();
        assert!(matches!(response, SystemResponse::Continue));
        assert_eq!(system.state(), SystemState::InfoDisplay);
    }

    #[test]
    fn test_command_execution() {
        let mut system = MinibufferSystem::new();

        // quit コマンドのテスト
        let response = system.handle_execute_result("quit".to_string()).unwrap();
        assert!(matches!(response, SystemResponse::Quit));

        // find-file コマンドのテスト
        let response = system.handle_execute_result("find-file test.txt".to_string()).unwrap();
        assert!(matches!(response, SystemResponse::FileOperation(FileOperation::Open(_))));

        // save-buffer コマンドのテスト
        let response = system.handle_execute_result("save-buffer".to_string()).unwrap();
        assert!(matches!(response, SystemResponse::FileOperation(FileOperation::Save)));

        // eval-expression コマンドのテスト（直接評価）
        let response = system.handle_execute_result("eval-expression (+ 1 2)".to_string()).unwrap();
        assert!(matches!(response, SystemResponse::Continue));
        assert_eq!(system.state(), SystemState::InfoDisplay);
    }

    #[test]
    fn test_deactivation() {
        let mut system = MinibufferSystem::new();

        // アクティブ化
        system.start_find_file(None).unwrap();
        assert!(system.is_active());

        // 非アクティブ化
        system.deactivate();
        assert!(!system.is_active());
        assert_eq!(system.state(), SystemState::Inactive);
    }

    #[test]
    fn test_add_command() {
        let mut system = MinibufferSystem::new();
        let initial_count = system.available_commands().len();

        system.add_command("custom-command".to_string());
        assert_eq!(system.available_commands().len(), initial_count + 1);
        assert!(system.available_commands().contains(&"custom-command".to_string()));
    }

    #[test]
    fn test_performance_stats() {
        let system = MinibufferSystem::new();
        let stats = system.performance_stats();

        assert_eq!(stats.completions_count, 0);
        assert_eq!(stats.history_size, 0);
        assert!(stats.uptime.as_nanos() > 0);
    }
}
