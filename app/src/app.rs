//! メインアプリケーション構造体
//!
//! アプリケーション全体の状態管理とメインループを実装

use crate::error::{AltreError, Result};
use crate::buffer::{BufferManager, TextEditor, CursorPosition, EditOperations};
use std::path::Path;

/// メインアプリケーション構造体
///
/// 全てのコンポーネントを統合し、アプリケーションのライフサイクルを管理
pub struct App {
    /// アプリケーション実行状態
    running: bool,
    /// 初期化状態
    initialized: bool,
    /// バッファマネージャー
    buffer_manager: BufferManager,
    /// メインエディタ
    editor: TextEditor,
    // 将来的に追加予定
    // ui_state: UiState,
    // input_handler: InputHandler,
    // minibuffer: MinibufferState,
}

impl App {
    /// 新しいアプリケーションインスタンスを作成
    pub fn new() -> Result<Self> {
        Ok(App {
            running: true,
            initialized: true,
            buffer_manager: BufferManager::new(),
            editor: TextEditor::new(),
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

    /// アプリケーションが初期化されているかを確認
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// ファイルを開く
    pub fn open_file(&mut self, file_path: &str) -> Result<()> {
        // 新しいバッファを作成
        let _buffer_id = self.buffer_manager.create_buffer();

        // ファイル読み込み（現在は仮実装）
        if Path::new(file_path).exists() {
            // TODO: 実際のファイル読み込み実装
            // let content = std::fs::read_to_string(file_path)
            //     .map_err(|e| AltreError::System(e.to_string()))?;
            // self.editor = TextEditor::from_str(&content);
        }

        Ok(())
    }

    /// バッファが存在するかを確認
    pub fn has_buffer(&self) -> bool {
        self.buffer_manager.current_buffer_id().is_some()
    }

    /// 文字を挿入
    pub fn insert_char(&mut self, ch: char) -> Result<()> {
        self.editor.insert_char(ch)
    }

    /// バッファの内容を取得
    pub fn get_buffer_content(&self) -> String {
        self.editor.to_string()
    }

    /// 文字列を挿入
    pub fn insert_str(&mut self, s: &str) -> Result<()> {
        self.editor.insert_str(s)
    }

    /// カーソルを開始位置に移動
    pub fn move_cursor_to_start(&mut self) -> Result<()> {
        let start_position = CursorPosition::new();
        self.editor.set_cursor(start_position);
        Ok(())
    }

    /// カーソル位置を取得
    pub fn get_cursor_position(&self) -> &CursorPosition {
        self.editor.cursor()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("アプリケーションの初期化に失敗しました")
    }
}