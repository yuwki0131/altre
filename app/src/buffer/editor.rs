//! エディタ操作インターフェース
//!
//! 基本編集操作のメインインターフェース

use crate::buffer::{gap_buffer::GapBuffer, cursor::CursorPosition, navigation::{NavigationAction, NavigationError, NavigationSystem}};
use crate::error::{EditError, Result};
use std::time::Instant;

/// 変更イベント
#[derive(Debug, Clone)]
pub enum ChangeEvent {
    Insert {
        position: usize,
        content: String,
    },
    Delete {
        position: usize,
        content: String,
    },
    CursorMove {
        old_position: CursorPosition,
        new_position: CursorPosition,
    },
}

/// 編集操作インターフェース
pub trait EditOperations {
    /// 文字を挿入
    fn insert_char(&mut self, ch: char) -> Result<()>;

    /// 文字列を挿入
    fn insert_str(&mut self, s: &str) -> Result<()>;

    /// カーソル位置に文字を挿入
    fn insert_char_at_cursor(&mut self, ch: char) -> Result<()>;

    /// Backspace削除（カーソル前削除）
    fn delete_backward(&mut self) -> Result<char>;

    /// Delete削除（カーソル後削除）
    fn delete_forward(&mut self) -> Result<char>;

    /// 改行を挿入
    fn insert_newline(&mut self) -> Result<()>;

    /// 範囲削除
    fn delete_range(&mut self, start: usize, end: usize) -> Result<String>;
}

/// 変更通知リスナー
pub trait ChangeListener {
    fn on_change(&mut self, event: &ChangeEvent);
}

/// 変更通知システム
pub struct ChangeNotifier {
    listeners: Vec<Box<dyn ChangeListener>>,
}

impl ChangeNotifier {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    /// リスナーを追加
    pub fn add_listener(&mut self, listener: Box<dyn ChangeListener>) {
        self.listeners.push(listener);
    }

    /// 変更を通知
    pub fn notify(&mut self, event: ChangeEvent) {
        for listener in &mut self.listeners {
            listener.on_change(&event);
        }
    }
}

/// テキストエディタのメイン構造体
pub struct TextEditor {
    /// テキストバッファ
    buffer: GapBuffer,
    /// カーソル位置
    cursor: CursorPosition,
    /// ナビゲーションシステム
    navigation: NavigationSystem,
    /// 変更通知システム
    change_notifier: ChangeNotifier,
    /// 最後の操作時刻（パフォーマンス監視用）
    last_operation_time: Instant,
}

impl TextEditor {
    /// 新しいエディタを作成
    pub fn new() -> Self {
        Self {
            buffer: GapBuffer::new(),
            cursor: CursorPosition::new(),
            navigation: NavigationSystem::new(),
            change_notifier: ChangeNotifier::new(),
            last_operation_time: Instant::now(),
        }
    }

    /// 文字列からエディタを作成
    pub fn from_str(s: &str) -> Self {
        Self {
            buffer: GapBuffer::from_str(s),
            cursor: CursorPosition::new(),
            navigation: NavigationSystem::new(),
            change_notifier: ChangeNotifier::new(),
            last_operation_time: Instant::now(),
        }
    }

    /// バッファの内容を文字列として取得
    pub fn to_string(&self) -> String {
        self.buffer.to_string()
    }

    /// カーソル位置を取得
    pub fn cursor(&self) -> &CursorPosition {
        &self.cursor
    }

    /// ナビゲーションシステムへの参照
    pub fn navigation(&self) -> &NavigationSystem {
        &self.navigation
    }

    /// カーソル位置を設定
    pub fn set_cursor(&mut self, position: CursorPosition) {
        let old_position = self.cursor;
        self.cursor = position;
        self.clamp_cursor_position();
        let _ = self.sync_navigation_cursor();

        // カーソル移動を通知
        self.change_notifier.notify(ChangeEvent::CursorMove {
            old_position,
            new_position: self.cursor,
        });
    }

    /// 変更リスナーを追加
    pub fn add_change_listener(&mut self, listener: Box<dyn ChangeListener>) {
        self.change_notifier.add_listener(listener);
    }

    /// 有効な入力文字かどうかを判定
    fn is_valid_input_char(&self, ch: char) -> bool {
        match ch {
            // 制御文字は除外（改行は別途処理）
            '\u{0000}'..='\u{001F}' => false,
            '\u{007F}' => false, // DEL
            // 印刷可能文字とスペース、タブは有効
            _ => !ch.is_control() || ch == '\t'
        }
    }

    /// カーソル位置をギャップバッファと同期
    fn sync_cursor_with_buffer(&mut self) {
        let text = self.buffer.to_string();

        // カーソル位置の有効性をチェック
        if self.cursor.char_pos > self.buffer.len_chars() {
            self.cursor.char_pos = self.buffer.len_chars();
        }

        // 行・列情報を再計算
        self.recalculate_cursor_line_column(&text);
        let _ = self.sync_navigation_cursor();
    }

    /// カーソルの行・列位置を再計算
    fn recalculate_cursor_line_column(&mut self, text: &str) {
        let mut line = 0;
        let mut column = 0;

        for (i, ch) in text.chars().enumerate() {
            if i == self.cursor.char_pos {
                break;
            }

            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        self.cursor.line = line;
        self.cursor.column = column;
    }

    /// カーソル移動の境界値チェック
    fn clamp_cursor_position(&mut self) {
        let max_pos = self.buffer.len_chars();

        if self.cursor.char_pos > max_pos {
            self.cursor.char_pos = max_pos;
        }

        // 行・列の境界値も調整
        let text = self.buffer.to_string();
        self.clamp_cursor_line_column(&text);
    }

    /// 行・列位置の境界値チェック
    fn clamp_cursor_line_column(&mut self, text: &str) {
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            self.cursor.line = 0;
            self.cursor.column = 0;
            return;
        }

        // 行数の境界値
        if self.cursor.line >= lines.len() {
            self.cursor.line = lines.len() - 1;
        }

        // 列数の境界値
        if self.cursor.line < lines.len() {
            let line_length = lines[self.cursor.line].chars().count();
            if self.cursor.column > line_length {
                self.cursor.column = line_length;
            }
        }
    }

    /// エラー時の安全な状態復旧
    fn recover_from_error(&mut self, error: &crate::error::AltreError) -> Result<()> {
        match error {
            crate::error::AltreError::Edit(_) => {
                // カーソル位置を安全な位置に修正
                self.clamp_cursor_position();
                Ok(())
            }
            crate::error::AltreError::Buffer(_) => {
                // バッファとカーソルの整合性を回復
                self.sync_cursor_with_buffer();
                Ok(())
            }
            crate::error::AltreError::System(_) => {
                // 致命的エラー：QA.mdに従い即座に終了
                Err(error.clone())
            }
            _ => Ok(())
        }
    }

    /// 操作の安全実行
    fn safe_execute<F, T>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut Self) -> Result<T>,
    {
        // 現在の状態を保存
        let saved_cursor = self.cursor;

        match operation(self) {
            Ok(result) => Ok(result),
            Err(error) => {
                // エラー時は状態を復旧
                self.cursor = saved_cursor;
                // recover_from_errorは実行するが、元のエラーを返す
                let _ = self.recover_from_error(&error);
                Err(error)
            }
        }
    }

    /// 改行コードの正規化
    fn normalize_line_ending(&self, input: &str) -> String {
        input
            .replace("\r\n", "\n")  // Windows CRLF → LF
            .replace("\r", "\n")    // Mac CR → LF
    }

    /// ナビゲーション状態をカーソル位置と同期
    fn sync_navigation_cursor(&mut self) -> Result<()> {
        let text = self.buffer.to_string();
        self.navigation.set_cursor(self.cursor);
        self.navigation
            .recover_from_invalid_position(&text)
            .map_err(|e| EditError::BufferError(format!("ナビゲーション更新に失敗しました: {}", e)).into())
    }

    /// ナビゲーション操作の実行
    pub fn navigate(&mut self, action: NavigationAction) -> std::result::Result<bool, NavigationError> {
        let text = self.buffer.to_string();
        self.navigation.set_cursor(self.cursor);
        let moved = self.navigation.navigate(&text, action)?;
        if moved {
            let new_cursor = *self.navigation.cursor();
            let old_position = self.cursor;
            self.cursor = new_cursor;
            let _ = self.sync_navigation_cursor();
            self.change_notifier.notify(ChangeEvent::CursorMove {
                old_position,
                new_position: self.cursor,
            });
        }
        Ok(moved)
    }

    /// 挿入後のカーソル位置更新
    fn update_cursor_after_insert(&mut self, inserted: &str) {
        for ch in inserted.chars() {
            if ch == '\n' {
                self.cursor.line += 1;
                self.cursor.column = 0;
            } else {
                self.cursor.column += 1;
            }
        }
    }

    /// パフォーマンス計測開始
    fn start_performance_measurement(&mut self) {
        self.last_operation_time = Instant::now();
    }

    /// パフォーマンス計測終了とログ
    fn end_performance_measurement(&self, operation_name: &str) {
        let duration = self.last_operation_time.elapsed();
        if duration.as_millis() > 1 {
            eprintln!("Warning: {} took {}ms (target: <1ms)", operation_name, duration.as_millis());
        }
    }
}

impl EditOperations for TextEditor {
    /// 文字を挿入
    fn insert_char(&mut self, ch: char) -> Result<()> {
        self.start_performance_measurement();

        let result = self.safe_execute(|editor| {
            // 1. 入力文字の検証
            if !editor.is_valid_input_char(ch) {
                return Err(EditError::InvalidChar(ch).into());
            }

            // 2. カーソル位置の取得
            let cursor_pos = editor.cursor.char_pos;

            // 3. ギャップバッファに挿入
            editor.buffer.insert(cursor_pos, ch)
                .map_err(|_| EditError::BufferError("挿入失敗".to_string()))?;

            // 4. カーソル位置を更新
            editor.cursor.char_pos += 1;
            if ch == '\n' {
                editor.cursor.line += 1;
                editor.cursor.column = 0;
            } else {
                editor.cursor.column += 1;
            }

            // 5. 変更通知
            editor.change_notifier.notify(ChangeEvent::Insert {
                position: cursor_pos,
                content: ch.to_string(),
            });

            editor.sync_navigation_cursor()?;

            Ok(())
        });

        self.end_performance_measurement("insert_char");
        result
    }

    /// 文字列を挿入
    fn insert_str(&mut self, s: &str) -> Result<()> {
        self.start_performance_measurement();

        let result = self.safe_execute(|editor| {
            if s.is_empty() {
                return Ok(());
            }

            // 改行コードを正規化
            let normalized = editor.normalize_line_ending(s);

            let cursor_pos = editor.cursor.char_pos;

            // ギャップバッファに挿入
            editor.buffer.insert_str(cursor_pos, &normalized)
                .map_err(|_| EditError::BufferError("文字列挿入失敗".to_string()))?;

            // カーソル位置を更新
            let char_count = normalized.chars().count();
            editor.cursor.char_pos += char_count;
            editor.update_cursor_after_insert(&normalized);

            // 変更通知
            editor.change_notifier.notify(ChangeEvent::Insert {
                position: cursor_pos,
                content: normalized,
            });

            editor.sync_navigation_cursor()?;

            Ok(())
        });

        self.end_performance_measurement("insert_str");
        result
    }

    /// カーソル位置に文字を挿入
    fn insert_char_at_cursor(&mut self, ch: char) -> Result<()> {
        self.insert_char(ch)
    }

    /// Backspace削除（カーソル前削除）
    fn delete_backward(&mut self) -> Result<char> {
        self.start_performance_measurement();

        let result = self.safe_execute(|editor| {
            if editor.cursor.char_pos == 0 {
                return Err(EditError::AtBufferStart.into());
            }

            let pos = editor.cursor.char_pos - 1;
            let deleted_char = editor.buffer.delete(pos)
                .map_err(|_| EditError::BufferError("削除失敗".to_string()))?;

            // カーソルを後退
            editor.cursor.char_pos = pos;
            if deleted_char == '\n' && editor.cursor.line > 0 {
                editor.cursor.line -= 1;
                // 前の行の長さを計算してカラム位置を設定
                let text = editor.buffer.to_string();
                let lines: Vec<&str> = text.lines().collect();
                if editor.cursor.line < lines.len() {
                    editor.cursor.column = lines[editor.cursor.line].chars().count();
                }
            } else if editor.cursor.column > 0 {
                editor.cursor.column -= 1;
            }

            // 変更通知
            editor.change_notifier.notify(ChangeEvent::Delete {
                position: pos,
                content: deleted_char.to_string(),
            });

            editor.sync_navigation_cursor()?;

            Ok(deleted_char)
        });

        self.end_performance_measurement("delete_backward");
        result
    }

    /// Delete削除（カーソル後削除）
    fn delete_forward(&mut self) -> Result<char> {
        self.start_performance_measurement();

        let result = self.safe_execute(|editor| {
            if editor.cursor.char_pos >= editor.buffer.len_chars() {
                return Err(EditError::AtBufferEnd.into());
            }

            let pos = editor.cursor.char_pos;
            let deleted_char = editor.buffer.delete(pos)
                .map_err(|_| EditError::BufferError("削除失敗".to_string()))?;

            // カーソル位置は変更なし（文字が削除されたため相対的に正しい位置）

            // 変更通知
            editor.change_notifier.notify(ChangeEvent::Delete {
                position: pos,
                content: deleted_char.to_string(),
            });

            editor.sync_navigation_cursor()?;

            Ok(deleted_char)
        });

        self.end_performance_measurement("delete_forward");
        result
    }

    /// 改行を挿入
    fn insert_newline(&mut self) -> Result<()> {
        self.start_performance_measurement();

        let result = self.safe_execute(|editor| {
            let cursor_pos = editor.cursor.char_pos;

            // LF統一ポリシー
            editor.buffer.insert_str(cursor_pos, "\n")
                .map_err(|_| EditError::BufferError("改行挿入失敗".to_string()))?;

            // カーソルを次の行の先頭に移動
            editor.cursor.char_pos += 1;
            editor.cursor.line += 1;
            editor.cursor.column = 0;

            // 変更通知
            editor.change_notifier.notify(ChangeEvent::Insert {
                position: cursor_pos,
                content: "\n".to_string(),
            });

            editor.sync_navigation_cursor()?;

            Ok(())
        });

        self.end_performance_measurement("insert_newline");
        result
    }

    /// 範囲削除
    fn delete_range(&mut self, start: usize, end: usize) -> Result<String> {
        self.start_performance_measurement();

        let result = self.safe_execute(|editor| {
            if start > end {
                return Err(EditError::OutOfBounds(start).into());
            }

            let deleted_text = editor.buffer.delete_range(start, end)
                .map_err(|_| EditError::BufferError("範囲削除失敗".to_string()))?;

            // カーソル位置を調整
            if editor.cursor.char_pos > start {
                if editor.cursor.char_pos <= end {
                    editor.cursor.char_pos = start;
                } else {
                    editor.cursor.char_pos -= end - start;
                }
            }

            // カーソルの行・列を再計算
            editor.sync_cursor_with_buffer();

            // 変更通知
            editor.change_notifier.notify(ChangeEvent::Delete {
                position: start,
                content: deleted_text.clone(),
            });

            editor.sync_navigation_cursor()?;

            Ok(deleted_text)
        });

        self.end_performance_measurement("delete_range");
        result
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_char_insertion() {
        let mut editor = TextEditor::new();

        // 基本文字挿入
        assert!(editor.insert_char('a').is_ok());
        assert_eq!(editor.to_string(), "a");
        assert_eq!(editor.cursor.char_pos, 1);
    }

    #[test]
    fn test_utf8_char_insertion() {
        let mut editor = TextEditor::new();

        // 日本語文字挿入
        assert!(editor.insert_char('あ').is_ok());
        assert_eq!(editor.to_string(), "あ");
        assert_eq!(editor.cursor.char_pos, 1);
    }

    #[test]
    fn test_backspace_deletion() {
        let mut editor = TextEditor::from_str("hello");
        editor.cursor.char_pos = 5;

        let deleted = editor.delete_backward().unwrap();
        assert_eq!(deleted, 'o');
        assert_eq!(editor.to_string(), "hell");
        assert_eq!(editor.cursor.char_pos, 4);
    }

    #[test]
    fn test_delete_forward() {
        let mut editor = TextEditor::from_str("hello");
        editor.cursor.char_pos = 0;

        let deleted = editor.delete_forward().unwrap();
        assert_eq!(deleted, 'h');
        assert_eq!(editor.to_string(), "ello");
        assert_eq!(editor.cursor.char_pos, 0);
    }

    #[test]
    fn test_newline_insertion() {
        let mut editor = TextEditor::from_str("line1");
        editor.cursor.char_pos = 5;

        assert!(editor.insert_newline().is_ok());
        assert_eq!(editor.to_string(), "line1\n");
        assert_eq!(editor.cursor.line, 1);
        assert_eq!(editor.cursor.column, 0);
    }

    #[test]
    fn test_string_insertion() {
        let mut editor = TextEditor::new();

        assert!(editor.insert_str("hello world").is_ok());
        assert_eq!(editor.to_string(), "hello world");
        assert_eq!(editor.cursor.char_pos, 11);
    }

    #[test]
    fn test_line_ending_normalization() {
        let mut editor = TextEditor::new();

        // Windows CRLF
        assert!(editor.insert_str("line1\r\nline2").is_ok());
        assert_eq!(editor.to_string(), "line1\nline2");

        // Mac CR
        let mut editor2 = TextEditor::new();
        assert!(editor2.insert_str("line1\rline2").is_ok());
        assert_eq!(editor2.to_string(), "line1\nline2");
    }

    #[test]
    fn test_cursor_boundary_handling() {
        let mut editor = TextEditor::from_str("test");

        // 範囲外のカーソル位置を設定
        editor.cursor.char_pos = 100;
        editor.clamp_cursor_position();
        assert_eq!(editor.cursor.char_pos, 4);
    }

    #[test]
    fn test_delete_range() {
        let mut editor = TextEditor::from_str("hello world");
        editor.cursor.char_pos = 5;

        let deleted = editor.delete_range(0, 5).unwrap();
        assert_eq!(deleted, "hello");
        assert_eq!(editor.to_string(), " world");
        assert_eq!(editor.cursor.char_pos, 0);
    }

    #[test]
    fn test_invalid_char_rejection() {
        let mut editor = TextEditor::new();

        // 制御文字は拒否
        assert!(editor.insert_char('\u{0001}').is_err());
        assert!(editor.insert_char('\u{007F}').is_err());
    }

    #[test]
    fn test_boundary_deletions() {
        let mut editor = TextEditor::new();

        // 空のバッファでの削除
        assert!(editor.delete_backward().is_err());
        assert!(editor.delete_forward().is_err());

        // 1文字のバッファ
        editor.insert_char('a').unwrap();
        editor.cursor.char_pos = 0;
        assert!(editor.delete_backward().is_err());

        editor.cursor.char_pos = 1;
        assert!(editor.delete_forward().is_err());
    }
}
