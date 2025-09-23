//! ミニバッファ テストスイート
//!
//! - コア入力、履歴、補完、エラー表示、ファイル操作フローを網羅
//! - 実行コマンド: `cargo test minibuffer --offline`
//! - 既知の制約: メッセージ自動消去は実時間に依存するため、テストではキー入力での消去を検証

use altre::input::keybinding::{Key, KeyCode, KeyModifiers};
use altre::minibuffer::{
    FileOperation, MinibufferConfig, MinibufferMode, MinibufferState, MinibufferSystem,
    SystemEvent, SystemResponse,
};
pub use altre::minibuffer::{MinibufferMode, SystemResponse};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub mod unit_tests;
pub mod history_tests;
pub mod completion_tests;
pub mod integration_tests;
pub mod error_handling_tests;

/// ミニバッファテスト用ユーティリティ
pub struct MinibufferTestHelper {
    system: MinibufferSystem,
    temp_dir: Option<TempDir>,
}

impl MinibufferTestHelper {
    /// 既定設定でヘルパーを作成
    pub fn new() -> Self {
        Self {
            system: MinibufferSystem::new(),
            temp_dir: None,
        }
    }

    /// 設定を指定してヘルパーを作成
    pub fn with_config(config: MinibufferConfig) -> Self {
        Self {
            system: MinibufferSystem::with_config(config),
            temp_dir: None,
        }
    }

    /// 現在の低レベル状態を取得
    pub fn state(&self) -> &MinibufferState {
        self.system.minibuffer_state()
    }

    /// 現在のモードを取得
    pub fn mode(&self) -> MinibufferMode {
        self.state().mode.clone()
    }

    /// 現在の入力文字列
    pub fn input(&self) -> &str {
        self.system.current_input()
    }

    /// 現在の補完候補
    pub fn completions(&self) -> &[String] {
        self.system.completions()
    }

    /// 現在選択中の補完インデックス
    pub fn selected_completion(&self) -> Option<usize> {
        self.system.selected_completion()
    }

    /// Find-file モードを開始
    pub fn start_find_file(&mut self, initial: Option<&str>) {
        self.system
            .start_find_file(initial)
            .expect("failed to start find-file mode");
    }

    /// Execute-command モードを開始
    pub fn start_execute_command(&mut self) {
        self.system
            .start_execute_command()
            .expect("failed to start execute-command mode");
    }

    /// 1キー入力を送出
    pub fn send_key(&mut self, key: Key) -> SystemResponse {
        self.system
            .handle_event(SystemEvent::KeyInput(key))
            .expect("key handling failed")
    }

    /// 文字列を順に入力
    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            let response = self.send_key(key_char(ch));
            assert!(matches!(response, SystemResponse::Continue | SystemResponse::None));
        }
    }

    /// Enter キー
    pub fn press_enter(&mut self) -> SystemResponse {
        self.send_key(key_plain(KeyCode::Enter))
    }

    /// Backspace キー
    pub fn press_backspace(&mut self) -> SystemResponse {
        self.send_key(key_plain(KeyCode::Backspace))
    }

    /// Delete キー
    pub fn press_delete(&mut self) -> SystemResponse {
        self.send_key(key_plain(KeyCode::Delete))
    }

    /// Tab キー
    pub fn press_tab(&mut self) -> SystemResponse {
        self.send_key(key_plain(KeyCode::Tab))
    }

    /// Ctrl+<char> を送出
    pub fn press_ctrl(&mut self, ch: char) -> SystemResponse {
        self.send_key(key_ctrl(ch))
    }

    /// 矢印キー送出
    pub fn press_arrow(&mut self, code: KeyCode) -> SystemResponse {
        self.send_key(key_plain(code))
    }

    /// 一時ディレクトリにテストファイルを作成
    pub fn prepare_temp_dir<P: AsRef<Path>>(&mut self, files: &[P]) -> PathBuf {
        let temp = TempDir::new().expect("tempdir");
        for path in files {
            let absolute = temp.path().join(path.as_ref());
            if let Some(parent) = absolute.parent() {
                fs::create_dir_all(parent).expect("create parent directories");
            }
            fs::write(&absolute, b"test").expect("write fixture");
        }
        let path = temp.path().to_path_buf();
        self.temp_dir = Some(temp);
        path
    }

    /// 内部システムへの直接アクセス（高度な検証用）
    pub fn system(&mut self) -> &mut MinibufferSystem {
        &mut self.system
    }

    /// 直近の tempdir をクローン（存在しない場合は panic）
    pub fn temp_dir_path(&self) -> PathBuf {
        self.temp_dir
            .as_ref()
            .expect("tempdir not initialised")
            .path()
            .to_path_buf()
    }

    /// 直近の tempdir を破棄（明示的クリーンアップ用）
    pub fn clear_temp_dir(&mut self) {
        self.temp_dir = None;
    }
}

impl Default for MinibufferTestHelper {
    fn default() -> Self {
        Self::new()
    }
}

/// 修飾なしキー生成
pub fn key_plain(code: KeyCode) -> Key {
    Key {
        code,
        modifiers: KeyModifiers {
            ctrl: false,
            alt: false,
            shift: false,
        },
    }
}

/// 文字キー生成
pub fn key_char(c: char) -> Key {
    key_plain(KeyCode::Char(c))
}

/// Ctrl+文字キー生成
pub fn key_ctrl(c: char) -> Key {
    Key {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers {
            ctrl: true,
            alt: false,
            shift: false,
        },
    }
}

/// テスト用 Unicode 文字列セット
pub fn unicode_samples() -> &'static [&'static str] {
    &[
        "こんにちは",
        "你好",
        "안녕하세요",
        "🚀🌟💻",
        "naïve",
        "café",
    ]
}

/// 長いパス入力サンプル
pub fn long_path_input() -> String {
    let dir = "a".repeat(120);
    let file = "b".repeat(80);
    format!("{}/{}.txt", dir, file)
}

/// 異常系パスサンプル
pub fn invalid_path_samples() -> &'static [&'static str] {
    &[
        "",
        "/definitely/not/exist",
        "/root/permission_denied",
    ]
}

/// SystemResponse からファイル操作を抽出（ユーティリティ）
pub fn as_file_operation(response: SystemResponse) -> Option<FileOperation> {
    if let SystemResponse::FileOperation(op) = response {
        Some(op)
    } else {
        None
    }
}

/// SystemResponse が継続かどうか
pub fn is_continue(response: &SystemResponse) -> bool {
    matches!(response, SystemResponse::Continue | SystemResponse::None)
}
