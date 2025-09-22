//! ミニバッファテストスイート
//!
//! ミニバッファのコア機能・補完・履歴・エラーメッセージをカバーする包括的なテストスイート

use altre::minibuffer::{MinibufferSystem, MinibufferConfig, MinibufferState, MinibufferMode};
use altre::input::keybinding::{Key, KeyCode};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;

pub mod unit_tests;
pub mod history_tests;
pub mod completion_tests;
pub mod integration_tests;
pub mod error_handling_tests;

/// テスト用ヘルパー構造体
pub struct MinibufferTestHelper {
    pub system: MinibufferSystem,
    pub temp_dir: Option<TempDir>,
}

impl MinibufferTestHelper {
    /// 新しいテストヘルパーを作成
    pub fn new() -> Self {
        let config = MinibufferConfig::default();
        Self {
            system: MinibufferSystem::new(config),
            temp_dir: None,
        }
    }

    /// カスタム設定でテストヘルパーを作成
    pub fn with_config(config: MinibufferConfig) -> Self {
        Self {
            system: MinibufferSystem::new(config),
            temp_dir: None,
        }
    }

    /// 一時ディレクトリを作成し、テストファイルを準備
    pub fn with_temp_files(&mut self, files: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;

        // テストファイルを作成
        for file in files {
            let file_path = temp_dir.path().join(file);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, format!("Test content for {}", file))?;
        }

        self.temp_dir = Some(temp_dir);
        Ok(())
    }

    /// 一時ディレクトリのパスを取得
    pub fn temp_dir_path(&self) -> Option<PathBuf> {
        self.temp_dir.as_ref().map(|d| d.path().to_path_buf())
    }

    /// キーシーケンスをシミュレート
    pub fn simulate_keys(&mut self, keys: &[Key]) -> Vec<String> {
        let mut results = Vec::new();

        for key in keys {
            if let Ok(response) = self.system.handle_key_input(*key) {
                if let Some(message) = response.message {
                    results.push(message);
                }
            }
        }

        results
    }

    /// 文字列入力をシミュレート
    pub fn simulate_input(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        for ch in text.chars() {
            let key = Key::Char(ch);
            self.system.handle_key_input(key)?;
        }
        Ok(())
    }

    /// 現在の状態を取得
    pub fn state(&self) -> &MinibufferState {
        self.system.state()
    }

    /// キャンセルキー（C-g）をシミュレート
    pub fn simulate_cancel(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let cancel_key = Key::Ctrl('g');
        self.system.handle_key_input(cancel_key)?;
        Ok(())
    }

    /// タブ補完をシミュレート
    pub fn simulate_tab_completion(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let tab_key = Key::Tab;
        let response = self.system.handle_key_input(tab_key)?;
        Ok(self.state().completions.clone())
    }

    /// 履歴ナビゲーション（上）をシミュレート
    pub fn simulate_history_previous(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let key = Key::Ctrl('p');
        self.system.handle_key_input(key)?;
        Ok(())
    }

    /// 履歴ナビゲーション（下）をシミュレート
    pub fn simulate_history_next(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let key = Key::Ctrl('n');
        self.system.handle_key_input(key)?;
        Ok(())
    }

    /// Enterキーをシミュレート（コマンド実行）
    pub fn simulate_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let key = Key::Return;
        self.system.handle_key_input(key)?;
        Ok(())
    }

    /// バックスペースをシミュレート
    pub fn simulate_backspace(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let key = Key::Backspace;
        self.system.handle_key_input(key)?;
        Ok(())
    }

    /// ファイル選択モードを開始
    pub fn start_find_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.system.activate_find_file_mode()?;
        Ok(())
    }

    /// コマンド実行モードを開始
    pub fn start_execute_command(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.system.activate_execute_command_mode()?;
        Ok(())
    }

    /// エラーメッセージ表示をシミュレート
    pub fn simulate_error(&mut self, message: &str) {
        let expires_at = Instant::now() + Duration::from_secs(5);
        self.system.show_error_message(message.to_string(), expires_at);
    }

    /// メッセージ期限切れをシミュレート
    pub fn simulate_message_timeout(&mut self) {
        // 状態を強制的に非アクティブに変更
        if let MinibufferMode::ErrorDisplay { .. } | MinibufferMode::InfoDisplay { .. } = self.state().mode {
            self.system.deactivate();
        }
    }
}

impl Default for MinibufferTestHelper {
    fn default() -> Self {
        Self::new()
    }
}

/// テスト用のキー作成ヘルパー
pub fn char_key(c: char) -> Key {
    Key::Char(c)
}

pub fn ctrl_key(c: char) -> Key {
    Key::Ctrl(c)
}

/// Unicode文字のテストヘルパー
pub fn unicode_test_strings() -> Vec<&'static str> {
    vec![
        "こんにちは", // 日本語ひらがな
        "你好",       // 中国語
        "안녕하세요", // 韓国語
        "🚀🌟💻",    // 絵文字
        "café",       // アクセント付き文字
        "naïve",      // ダイアクリティカルマーク
    ]
}

/// パス長境界テストのヘルパー
pub fn long_path_test() -> String {
    let long_dir = "a".repeat(100);
    let long_file = "b".repeat(100);
    format!("{}/{}.txt", long_dir, long_file)
}

/// 無効なパスのテストケース
pub fn invalid_paths() -> Vec<&'static str> {
    vec![
        "", // 空のパス
        "/nonexistent/path/to/file.txt", // 存在しないパス
        "/root/restricted_file.txt", // 権限不足（通常）
        "file\0with\0null.txt", // null文字を含むパス
    ]
}