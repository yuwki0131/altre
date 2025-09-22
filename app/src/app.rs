//! メインアプリケーション構造体
//!
//! アプリケーション全体の状態管理とメインループを実装

use crate::error::{AltreError, Result};

/// メインアプリケーション構造体
///
/// 全てのコンポーネントを統合し、アプリケーションのライフサイクルを管理
pub struct App {
    /// アプリケーション実行状態
    running: bool,
    // 将来的に各モジュールのマネージャーを追加
    // buffer_manager: BufferManager,
    // ui_state: UiState,
    // input_handler: InputHandler,
    // minibuffer: MinibufferState,
}

impl App {
    /// 新しいアプリケーションインスタンスを作成
    pub fn new() -> Result<Self> {
        Ok(App {
            running: true,
        })
    }

    /// メインイベントループを実行
    pub fn run(&mut self) -> Result<()> {
        // TODO: 実際のイベントループ実装
        // 現在はプレースホルダー
        println!("altre MVP starting...");
        println!("Press Enter to exit");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .map_err(AltreError::from)?;

        self.running = false;
        Ok(())
    }

    /// アプリケーションが実行中かどうかを確認
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// アプリケーションを終了状態にする
    pub fn shutdown(&mut self) {
        self.running = false;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("アプリケーションの初期化に失敗しました")
    }
}