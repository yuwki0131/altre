//! プロンプト管理
//!
//! ユーザーからの入力を受け付けるプロンプトシステム

use crate::error::{AltreError, Result};
use super::completion::{CompletionEngine, PathCompletion};

/// プロンプトの結果
#[derive(Debug, Clone)]
pub enum PromptResult {
    /// 入力が完了した
    Completed(String),
    /// 入力がキャンセルされた
    Cancelled,
    /// 入力継続中
    InProgress,
}

/// プロンプトの種類
#[derive(Debug, Clone)]
pub enum PromptType {
    /// 単純なテキスト入力
    Text,
    /// ファイルパス入力（補完機能付き）
    FilePath,
    /// コマンド入力（補完機能付き）
    Command,
}

/// プロンプト管理器
pub struct PromptManager {
    /// 現在のプロンプトメッセージ
    message: String,
    /// 現在の入力内容
    input: String,
    /// プロンプトの種類
    prompt_type: PromptType,
    /// 補完エンジン
    completion_engine: Option<Box<dyn CompletionEngine>>,
    /// アクティブかどうか
    active: bool,
    /// 補完候補
    completion_candidates: Vec<String>,
    /// 補完状態
    completion_active: bool,
}

impl PromptManager {
    /// 新しいプロンプト管理器を作成
    pub fn new() -> Self {
        Self {
            message: String::new(),
            input: String::new(),
            prompt_type: PromptType::Text,
            completion_engine: None,
            active: false,
            completion_candidates: Vec::new(),
            completion_active: false,
        }
    }

    /// テキストプロンプトを開始
    pub fn start_text_prompt(&mut self, message: String) {
        self.message = message;
        self.input.clear();
        self.prompt_type = PromptType::Text;
        self.completion_engine = None;
        self.active = true;
        self.reset_completion();
    }

    /// ファイルパスプロンプトを開始
    pub fn start_file_prompt(&mut self, message: String) {
        self.message = message;
        self.input.clear();
        self.prompt_type = PromptType::FilePath;
        self.completion_engine = Some(Box::new(PathCompletion::new()));
        self.active = true;
        self.reset_completion();
    }

    /// コマンドプロンプトを開始
    pub fn start_command_prompt(&mut self, message: String) {
        self.message = message;
        self.input.clear();
        self.prompt_type = PromptType::Command;
        self.completion_engine = Some(Box::new(super::completion::CommandCompletion::new()));
        self.active = true;
        self.reset_completion();
    }

    /// プロンプトを終了
    pub fn end_prompt(&mut self) {
        self.active = false;
        self.message.clear();
        self.input.clear();
        self.completion_engine = None;
        self.reset_completion();
    }

    /// アクティブかどうか
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// プロンプトメッセージを取得
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 現在の入力を取得
    pub fn input(&self) -> &str {
        &self.input
    }

    /// 文字を追加
    pub fn add_char(&mut self, ch: char) -> Result<()> {
        if !self.active {
            return Err(AltreError::Application("プロンプトがアクティブではありません".to_string()));
        }

        self.input.push(ch);
        self.reset_completion();
        Ok(())
    }

    /// 最後の文字を削除
    pub fn backspace(&mut self) -> Result<bool> {
        if !self.active {
            return Err(AltreError::Application("プロンプトがアクティブではありません".to_string()));
        }

        if self.input.is_empty() {
            Ok(false)
        } else {
            self.input.pop();
            self.reset_completion();
            Ok(true)
        }
    }

    /// 入力を完了
    pub fn complete_input(&mut self) -> PromptResult {
        if !self.active {
            return PromptResult::Cancelled;
        }

        let result = self.input.clone();
        self.end_prompt();
        PromptResult::Completed(result)
    }

    /// 入力をキャンセル
    pub fn cancel_input(&mut self) -> PromptResult {
        self.end_prompt();
        PromptResult::Cancelled
    }

    /// 補完を実行
    pub fn complete(&mut self) -> Result<bool> {
        if !self.active {
            return Ok(false);
        }

        let Some(ref engine) = self.completion_engine else {
            return Ok(false);
        };

        // 補完候補を取得
        self.completion_candidates = engine.complete(&self.input)?;

        if self.completion_candidates.is_empty() {
            return Ok(false);
        }

        if self.completion_candidates.len() == 1 {
            // 単一候補の場合は自動補完
            let completion = &self.completion_candidates[0];
            self.input = engine.apply_completion(&self.input, completion);
            self.reset_completion();
            return Ok(true);
        }

        // 複数候補の場合は共通プレフィックスを補完
        let common = engine.common_prefix(&self.completion_candidates);
        if !common.is_empty() && common != self.input {
            self.input = common;
            self.completion_active = false;
            return Ok(true);
        }

        // 候補表示状態にする
        self.completion_active = true;
        Ok(true)
    }

    /// 補完状態をリセット
    fn reset_completion(&mut self) {
        self.completion_candidates.clear();
        self.completion_active = false;
    }

    /// 補完候補を取得
    pub fn completion_candidates(&self) -> &[String] {
        &self.completion_candidates
    }

    /// 補完が表示されているか
    pub fn is_completion_active(&self) -> bool {
        self.completion_active
    }

    /// プロンプト全体の表示文字列を生成
    pub fn display_string(&self) -> String {
        format!("{}{}", self.message, self.input)
    }

    /// カーソル位置を取得（プロンプトメッセージの長さ + 入力長）
    pub fn cursor_position(&self) -> usize {
        self.message.chars().count() + self.input.chars().count()
    }

    /// 入力を直接設定（テスト用）
    #[cfg(test)]
    pub fn set_input(&mut self, input: String) {
        self.input = input;
        self.reset_completion();
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// プロンプトユーティリティ
pub struct PromptUtils;

impl PromptUtils {
    /// ファイルパスの妥当性をチェック
    pub fn validate_file_path(path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(AltreError::Path("ファイルパスが空です".to_string()));
        }

        // 危険な文字をチェック
        if path.contains('\0') {
            return Err(AltreError::Path("ファイルパスに無効な文字が含まれています".to_string()));
        }

        Ok(())
    }

    /// 入力文字列をトリム
    pub fn trim_input(input: &str) -> String {
        input.trim().to_string()
    }

    /// 入力の履歴管理（将来の機能用）
    pub fn add_to_history(_input: &str, _prompt_type: &PromptType) {
        // TODO: 入力履歴の実装
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_manager_creation() {
        let manager = PromptManager::new();
        assert!(!manager.is_active());
        assert_eq!(manager.message(), "");
        assert_eq!(manager.input(), "");
    }

    #[test]
    fn test_text_prompt() {
        let mut manager = PromptManager::new();
        manager.start_text_prompt("Enter text: ".to_string());

        assert!(manager.is_active());
        assert_eq!(manager.message(), "Enter text: ");
        assert!(matches!(manager.prompt_type, PromptType::Text));
    }

    #[test]
    fn test_file_prompt() {
        let mut manager = PromptManager::new();
        manager.start_file_prompt("Find file: ".to_string());

        assert!(manager.is_active());
        assert!(matches!(manager.prompt_type, PromptType::FilePath));
        assert!(manager.completion_engine.is_some());
    }

    #[test]
    fn test_input_manipulation() {
        let mut manager = PromptManager::new();
        manager.start_text_prompt("Enter: ".to_string());

        assert!(manager.add_char('a').is_ok());
        assert!(manager.add_char('b').is_ok());
        assert_eq!(manager.input(), "ab");

        assert!(manager.backspace().unwrap());
        assert_eq!(manager.input(), "a");

        let result = manager.complete_input();
        match result {
            PromptResult::Completed(input) => assert_eq!(input, "a"),
            _ => panic!("Expected completed result"),
        }

        assert!(!manager.is_active());
    }

    #[test]
    fn test_input_cancellation() {
        let mut manager = PromptManager::new();
        manager.start_text_prompt("Enter: ".to_string());
        manager.add_char('t').unwrap();

        let result = manager.cancel_input();
        match result {
            PromptResult::Cancelled => {},
            _ => panic!("Expected cancelled result"),
        }

        assert!(!manager.is_active());
    }

    #[test]
    fn test_display_string() {
        let mut manager = PromptManager::new();
        manager.start_text_prompt("Enter: ".to_string());
        manager.add_char('t').unwrap();
        manager.add_char('e').unwrap();

        assert_eq!(manager.display_string(), "Enter: te");
        assert_eq!(manager.cursor_position(), 9); // "Enter: te".chars().count()
    }

    #[test]
    fn test_path_validation() {
        assert!(PromptUtils::validate_file_path("valid/path.txt").is_ok());
        assert!(PromptUtils::validate_file_path("").is_err());
        assert!(PromptUtils::validate_file_path("invalid\0path").is_err());
    }

    #[test]
    fn test_input_trimming() {
        assert_eq!(PromptUtils::trim_input("  test  "), "test");
        assert_eq!(PromptUtils::trim_input("\tspaced\n"), "spaced");
    }
}