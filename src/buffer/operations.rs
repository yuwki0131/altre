//! 編集操作の定義と実装
//!
//! テキストバッファに対する各種編集操作を定義

// use crate::error::{AltreError, Result};  // 将来使用予定
use super::cursor::{CursorMovement, CursorMover};
use super::{Buffer, CursorPosition};

/// 編集操作の種類
#[derive(Debug, Clone)]
pub enum EditOperation {
    /// 文字挿入
    InsertChar { pos: usize, ch: char },
    /// 文字列挿入
    InsertString { pos: usize, text: String },
    /// 文字削除
    DeleteChar { pos: usize },
    /// 範囲削除
    DeleteRange { start: usize, end: usize },
    /// カーソル移動
    MoveCursor { movement: CursorMovement },
}

/// 編集操作の実行結果
#[derive(Debug)]
pub struct EditResult {
    /// 操作が成功したかどうか
    pub success: bool,
    /// エラーメッセージ（失敗時）
    pub error_message: Option<String>,
    /// カーソル位置の変更があったかどうか
    pub cursor_moved: bool,
    /// テキストの変更があったかどうか
    pub text_changed: bool,
}

impl EditResult {
    /// 成功結果を作成
    pub fn success(cursor_moved: bool, text_changed: bool) -> Self {
        Self {
            success: true,
            error_message: None,
            cursor_moved,
            text_changed,
        }
    }

    /// 失敗結果を作成
    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            error_message: Some(error.to_string()),
            cursor_moved: false,
            text_changed: false,
        }
    }
}

/// 編集操作実行エンジン
pub struct EditEngine;

impl EditEngine {
    /// 編集操作をバッファに適用
    pub fn apply_operation(buffer: &mut Buffer, operation: EditOperation) -> EditResult {
        match operation {
            EditOperation::InsertChar { pos, ch } => Self::insert_char(buffer, pos, ch),
            EditOperation::InsertString { pos, text } => Self::insert_string(buffer, pos, &text),
            EditOperation::DeleteChar { pos } => Self::delete_char(buffer, pos),
            EditOperation::DeleteRange { start, end } => Self::delete_range(buffer, start, end),
            EditOperation::MoveCursor { movement } => Self::move_cursor(buffer, movement),
        }
    }

    /// 文字挿入操作
    fn insert_char(buffer: &mut Buffer, pos: usize, ch: char) -> EditResult {
        match buffer.content.insert_char(pos, ch) {
            Ok(()) => {
                buffer.set_modified(true);
                // カーソルを挿入位置の後に移動
                let text = buffer.content.get_text();
                let mut cursor = buffer.cursor;
                cursor.char_pos = pos + 1;
                Self::update_cursor_line_column(&mut cursor, &text);
                buffer.cursor = cursor;

                EditResult::success(true, true)
            }
            Err(e) => EditResult::failure(&e.to_string()),
        }
    }

    /// 文字列挿入操作
    fn insert_string(buffer: &mut Buffer, pos: usize, text: &str) -> EditResult {
        match buffer.content.insert_str(pos, text) {
            Ok(()) => {
                buffer.set_modified(true);
                // カーソルを挿入テキストの後に移動
                let buffer_text = buffer.content.get_text();
                let mut cursor = buffer.cursor;
                cursor.char_pos = pos + text.chars().count();
                Self::update_cursor_line_column(&mut cursor, &buffer_text);
                buffer.cursor = cursor;

                EditResult::success(true, true)
            }
            Err(e) => EditResult::failure(&e.to_string()),
        }
    }

    /// 文字削除操作
    fn delete_char(buffer: &mut Buffer, pos: usize) -> EditResult {
        if pos >= buffer.content.char_len() {
            return EditResult::failure("削除位置が範囲外です");
        }

        match buffer.content.delete_char(pos) {
            Ok(()) => {
                buffer.set_modified(true);
                // カーソルを削除位置に移動
                let text = buffer.content.get_text();
                let mut cursor = buffer.cursor;
                cursor.char_pos = pos;
                Self::update_cursor_line_column(&mut cursor, &text);
                buffer.cursor = cursor;

                EditResult::success(true, true)
            }
            Err(e) => EditResult::failure(&e.to_string()),
        }
    }

    /// 範囲削除操作
    fn delete_range(buffer: &mut Buffer, start: usize, end: usize) -> EditResult {
        if start >= end {
            return EditResult::failure("無効な削除範囲です");
        }

        if end > buffer.content.char_len() {
            return EditResult::failure("削除範囲が範囲外です");
        }

        // 範囲を逆順で削除（位置のずれを防ぐため）
        for pos in (start..end).rev() {
            if let Err(e) = buffer.content.delete_char(pos) {
                return EditResult::failure(&e.to_string());
            }
        }

        buffer.set_modified(true);
        // カーソルを削除開始位置に移動
        let text = buffer.content.get_text();
        let mut cursor = buffer.cursor;
        cursor.char_pos = start;
        Self::update_cursor_line_column(&mut cursor, &text);
        buffer.cursor = cursor;

        EditResult::success(true, true)
    }

    /// カーソル移動操作
    fn move_cursor(buffer: &mut Buffer, movement: CursorMovement) -> EditResult {
        let text = buffer.content.get_text();
        let mut cursor = buffer.cursor;

        let moved = CursorMover::move_cursor(&mut cursor, &text, movement);

        if moved {
            buffer.cursor = cursor;
            EditResult::success(true, false)
        } else {
            EditResult::success(false, false)
        }
    }

    /// カーソルの行・列位置を文字位置から更新
    fn update_cursor_line_column(cursor: &mut CursorPosition, text: &str) {
        let chars: Vec<char> = text.chars().collect();
        let mut line = 0;
        let mut column = 0;

        for (i, &ch) in chars.iter().enumerate() {
            if i == cursor.char_pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        cursor.line = line;
        cursor.column = column;
    }
}

/// 編集操作のビルダーパターン
pub struct EditOperationBuilder;

impl EditOperationBuilder {
    /// 文字挿入操作を作成
    pub fn insert_char(pos: usize, ch: char) -> EditOperation {
        EditOperation::InsertChar { pos, ch }
    }

    /// 文字列挿入操作を作成
    pub fn insert_string(pos: usize, text: String) -> EditOperation {
        EditOperation::InsertString { pos, text }
    }

    /// 文字削除操作を作成
    pub fn delete_char(pos: usize) -> EditOperation {
        EditOperation::DeleteChar { pos }
    }

    /// 範囲削除操作を作成
    pub fn delete_range(start: usize, end: usize) -> EditOperation {
        EditOperation::DeleteRange { start, end }
    }

    /// カーソル移動操作を作成
    pub fn move_cursor(movement: CursorMovement) -> EditOperation {
        EditOperation::MoveCursor { movement }
    }
}

/// 編集操作の実行履歴（将来のアンドゥ機能用）
#[derive(Debug)]
pub struct EditHistory {
    operations: Vec<EditOperation>,
    current_index: usize,
}

impl EditHistory {
    /// 新しい編集履歴を作成
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            current_index: 0,
        }
    }

    /// 操作を履歴に追加
    pub fn push_operation(&mut self, operation: EditOperation) {
        // 現在位置以降の履歴を削除（新しい分岐を作成）
        self.operations.truncate(self.current_index);
        self.operations.push(operation);
        self.current_index = self.operations.len();
    }

    /// 履歴の長さを取得
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// 履歴が空かどうか
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// アンドゥ可能かどうか
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    /// リドゥ可能かどうか
    pub fn can_redo(&self) -> bool {
        self.current_index < self.operations.len()
    }
}

impl Default for EditHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::GapBuffer;

    #[test]
    fn test_insert_char_operation() {
        let mut buffer = Buffer::new();
        let operation = EditOperationBuilder::insert_char(0, 'A');
        let result = EditEngine::apply_operation(&mut buffer, operation);

        assert!(result.success);
        assert!(result.text_changed);
        assert!(result.cursor_moved);
        assert_eq!(buffer.content.get_text(), "A");
        assert_eq!(buffer.cursor.char_pos, 1);
    }

    #[test]
    fn test_insert_string_operation() {
        let mut buffer = Buffer::new();
        let operation = EditOperationBuilder::insert_string(0, "Hello".to_string());
        let result = EditEngine::apply_operation(&mut buffer, operation);

        assert!(result.success);
        assert!(result.text_changed);
        assert_eq!(buffer.content.get_text(), "Hello");
        assert_eq!(buffer.cursor.char_pos, 5);
    }

    #[test]
    fn test_delete_char_operation() {
        let mut buffer = Buffer::new();
        buffer.content = GapBuffer::from_str("Hello");

        let operation = EditOperationBuilder::delete_char(1);
        let result = EditEngine::apply_operation(&mut buffer, operation);

        assert!(result.success);
        assert!(result.text_changed);
        assert_eq!(buffer.content.get_text(), "Hllo");
        assert_eq!(buffer.cursor.char_pos, 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut buffer = Buffer::new();
        buffer.content = GapBuffer::from_str("Hello\nWorld");

        let operation = EditOperationBuilder::move_cursor(CursorMovement::Forward);
        let result = EditEngine::apply_operation(&mut buffer, operation);

        assert!(result.success);
        assert!(result.cursor_moved);
        assert!(!result.text_changed);
        assert_eq!(buffer.cursor.char_pos, 1);
    }

    #[test]
    fn test_edit_history() {
        let mut history = EditHistory::new();
        assert!(history.is_empty());
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        let operation = EditOperationBuilder::insert_char(0, 'A');
        history.push_operation(operation);

        assert_eq!(history.len(), 1);
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }
}
