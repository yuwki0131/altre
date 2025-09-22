//! ナビゲーションシステム
//!
//! ギャップバッファ上のカーソル移動を司る軽量ユーティリティ。

use crate::buffer::cursor::CursorPosition;
use std::cmp::min;

/// ナビゲーション時のエラー。
#[derive(Debug, thiserror::Error)]
pub enum NavigationError {
    #[error("cursor is already at the beginning of the buffer")]
    StartOfBuffer,
    #[error("cursor is already at the end of the buffer")]
    EndOfBuffer,
    #[error("line {0} is out of range")]
    InvalidLine(usize),
    #[error("internal navigation error: {0}")]
    Internal(String),
}

/// ナビゲーション操作の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationAction {
    MoveCharForward,
    MoveCharBackward,
    MoveLineUp,
    MoveLineDown,
    MoveLineStart,
    MoveLineEnd,
    MoveBufferStart,
    MoveBufferEnd,
}

/// 行・列を含むカーソル位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub char_pos: usize,
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(char_pos: usize, line: usize, column: usize) -> Self {
        Self { char_pos, line, column }
    }
}

/// テキストスナップショット。ナビゲーション演算に必要な行情報を保持する。
struct TextSnapshot {
    chars: Vec<char>,
    line_starts: Vec<usize>,
    line_lengths: Vec<usize>,
    #[allow(dead_code)]
    tab_width: usize,
}

impl TextSnapshot {
    fn new(text: &str) -> Self {
        Self::with_tab_width(text, 4) // デフォルトTab幅は4
    }

    fn with_tab_width(text: &str, tab_width: usize) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let mut line_starts = vec![0];
        let mut line_lengths = Vec::new();
        let mut current_len = 0usize;

        for (idx, ch) in chars.iter().enumerate() {
            if *ch == '\n' {
                line_lengths.push(current_len);
                line_starts.push(idx + 1);
                current_len = 0;
            } else {
                current_len += 1;
            }
        }

        line_lengths.push(current_len);
        // `line_starts` の最後は文字数と一致していない場合があるため調整する
        // ただし、ファイル末尾の追加の line_start は line_of_char の計算に影響するため追加しない

        Self { chars, line_starts, line_lengths, tab_width }
    }

    fn char_count(&self) -> usize {
        self.chars.len()
    }

    fn line_count(&self) -> usize {
        self.line_lengths.len()
    }

    fn line_start(&self, line: usize) -> Option<usize> {
        self.line_starts.get(line).copied()
    }

    fn line_length(&self, line: usize) -> Option<usize> {
        self.line_lengths.get(line).copied()
    }

    fn char_at(&self, idx: usize) -> Option<char> {
        self.chars.get(idx).copied()
    }

    fn line_of_char(&self, char_pos: usize) -> usize {
        // パフォーマンス向上のため二分探索を使用（O(log n)）
        if char_pos == 0 {
            return 0;
        }

        // char_pos以下の最大のline_startを見つける
        match self.line_starts.binary_search(&char_pos) {
            Ok(index) => index, // 正確な一致
            Err(index) => index.saturating_sub(1), // 挿入位置の前の要素
        }
    }

    fn char_pos_for_line_col(&self, line: usize, column: usize) -> Option<usize> {
        let start = self.line_start(line)?;
        let len = self.line_length(line)?;
        let clamped = min(column, len);
        Some(start + clamped)
    }

    /// 指定位置の表示幅を計算（Tab考慮）
    #[allow(dead_code)]
    fn display_width_at(&self, char_pos: usize) -> usize {
        if char_pos >= self.chars.len() {
            return 0;
        }

        let line = self.line_of_char(char_pos);
        let line_start = self.line_start(line).unwrap_or(0);
        let mut display_col = 0;

        for pos in line_start..char_pos {
            if let Some(ch) = self.char_at(pos) {
                if ch == '\t' {
                    display_col += self.tab_width - (display_col % self.tab_width);
                } else {
                    display_col += 1;
                }
            }
        }

        display_col
    }

    /// 行内の表示列から文字位置を計算（Tab考慮）
    #[allow(dead_code)]
    fn char_pos_for_display_column(&self, line: usize, display_column: usize) -> Option<usize> {
        let start = self.line_start(line)?;
        let len = self.line_length(line)?;
        let mut display_col = 0;

        for i in 0..len {
            if display_col >= display_column {
                return Some(start + i);
            }

            if let Some(ch) = self.char_at(start + i) {
                if ch == '\t' {
                    display_col += self.tab_width - (display_col % self.tab_width);
                } else {
                    display_col += 1;
                }
            }
        }

        Some(start + len)
    }
}

/// カーソル移動に関する状態。
#[derive(Debug, Clone)]
struct ExtendedCursor {
    position: CursorPosition,
    preferred_column: Option<usize>,
}

impl ExtendedCursor {
    fn new() -> Self {
        Self {
            position: CursorPosition::new(),
            preferred_column: None,
        }
    }

    fn set(&mut self, cursor: CursorPosition) {
        self.position = cursor;
        self.preferred_column = Some(cursor.column);
    }

    fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }
}

/// ナビゲーションシステム本体。
#[derive(Debug, Clone)]
pub struct NavigationSystem {
    cursor: CursorPosition,
    extended: ExtendedCursor,
}

impl NavigationSystem {
    pub fn new() -> Self {
        Self {
            cursor: CursorPosition::new(),
            extended: ExtendedCursor::new(),
        }
    }

    /// カーソルを取得
    pub fn cursor(&self) -> &CursorPosition {
        &self.cursor
    }

    /// カーソルを設定（エディタ側で同期する）
    pub fn set_cursor(&mut self, cursor: CursorPosition) {
        self.cursor = cursor;
        self.extended.set(cursor);
    }

    /// テキストとアクションに基づいてカーソルを移動する。
    pub fn navigate(&mut self, text: &str, action: NavigationAction) -> Result<bool, NavigationError> {
        self.navigate_with_tab_width(text, action, 4)
    }

    /// Tab幅を指定してカーソルを移動する。
    pub fn navigate_with_tab_width(&mut self, text: &str, action: NavigationAction, tab_width: usize) -> Result<bool, NavigationError> {
        let snapshot = TextSnapshot::with_tab_width(text, tab_width);
        if snapshot.line_count() == 0 {
            return Ok(false);
        }

        let moved = match action {
            NavigationAction::MoveCharForward => self.move_char_forward(&snapshot),
            NavigationAction::MoveCharBackward => self.move_char_backward(&snapshot),
            NavigationAction::MoveLineUp => self.move_line_up(&snapshot),
            NavigationAction::MoveLineDown => self.move_line_down(&snapshot),
            NavigationAction::MoveLineStart => self.move_line_start(&snapshot),
            NavigationAction::MoveLineEnd => self.move_line_end(&snapshot),
            NavigationAction::MoveBufferStart => self.move_buffer_start(&snapshot),
            NavigationAction::MoveBufferEnd => self.move_buffer_end(&snapshot),
        }?;

        if moved {
            self.extended.position = self.cursor;
        }

        Ok(moved)
    }

    pub fn recover_from_invalid_position(&mut self, text: &str) -> Result<(), NavigationError> {
        let snapshot = TextSnapshot::new(text);
        let max_pos = snapshot.char_count();
        self.cursor.char_pos = min(self.cursor.char_pos, max_pos);
        let line = snapshot.line_of_char(self.cursor.char_pos);
        let line_start = snapshot.line_start(line).unwrap_or(0);
        let line_len = snapshot.line_length(line).unwrap_or(0);
        self.cursor.line = line;
        self.cursor.column = min(self.cursor.char_pos.saturating_sub(line_start), line_len);
        self.extended.set(self.cursor);
        Ok(())
    }

    fn move_char_forward(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.char_pos >= snapshot.char_count() {
            return Err(NavigationError::EndOfBuffer);
        }
        let ch = snapshot.char_at(self.cursor.char_pos).ok_or_else(|| NavigationError::Internal("cursor out of bounds".into()))?;
        self.cursor.char_pos += 1;
        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 0;
            self.extended.preferred_column = Some(0);
        } else {
            self.cursor.column += 1;
            self.extended.preferred_column = Some(self.cursor.column);
        }
        Ok(true)
    }

    fn move_char_backward(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.char_pos == 0 {
            return Err(NavigationError::StartOfBuffer);
        }
        let prev_char = snapshot
            .char_at(self.cursor.char_pos - 1)
            .ok_or_else(|| NavigationError::Internal("cursor out of bounds".into()))?;
        self.cursor.char_pos -= 1;
        if prev_char == '\n' {
            if self.cursor.line > 0 {
                self.cursor.line -= 1;
                let len = snapshot.line_length(self.cursor.line).unwrap_or(0);
                self.cursor.column = len;
            }
        } else {
            self.cursor.column = self.cursor.column.saturating_sub(1);
        }
        self.extended.preferred_column = Some(self.cursor.column);
        Ok(true)
    }

    fn move_line_up(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.line == 0 {
            return Err(NavigationError::StartOfBuffer);
        }
        let preferred = self.extended.preferred_column.unwrap_or(self.cursor.column);
        let target_line = self.cursor.line - 1;
        let target_len = snapshot.line_length(target_line).unwrap_or(0);
        let new_column = min(preferred, target_len);
        let new_char_pos = snapshot
            .char_pos_for_line_col(target_line, new_column)
            .ok_or_else(|| NavigationError::InvalidLine(target_line))?;
        self.cursor.line = target_line;
        self.cursor.column = new_column;
        self.cursor.char_pos = new_char_pos;
        self.extended.preferred_column = Some(preferred);
        Ok(true)
    }

    fn move_line_down(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let last_line = snapshot.line_count().saturating_sub(1);
        if self.cursor.line >= last_line {
            return Err(NavigationError::EndOfBuffer);
        }
        let preferred = self.extended.preferred_column.unwrap_or(self.cursor.column);
        let target_line = min(self.cursor.line + 1, last_line);
        let target_len = snapshot.line_length(target_line).unwrap_or(0);
        let new_column = min(preferred, target_len);
        let new_char_pos = snapshot
            .char_pos_for_line_col(target_line, new_column)
            .ok_or_else(|| NavigationError::InvalidLine(target_line))?;
        self.cursor.line = target_line;
        self.cursor.column = new_column;
        self.cursor.char_pos = new_char_pos;
        self.extended.preferred_column = Some(preferred);
        Ok(true)
    }

    fn move_line_start(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let start = snapshot
            .char_pos_for_line_col(self.cursor.line, 0)
            .ok_or_else(|| NavigationError::InvalidLine(self.cursor.line))?;
        self.cursor.char_pos = start;
        self.cursor.column = 0;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_line_end(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let len = snapshot
            .line_length(self.cursor.line)
            .ok_or_else(|| NavigationError::InvalidLine(self.cursor.line))?;
        let pos = snapshot
            .char_pos_for_line_col(self.cursor.line, len)
            .ok_or_else(|| NavigationError::InvalidLine(self.cursor.line))?;
        self.cursor.char_pos = pos;
        self.cursor.column = len;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_buffer_start(&mut self, _snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        self.cursor.char_pos = 0;
        self.cursor.line = 0;
        self.cursor.column = 0;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_buffer_end(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let total_chars = snapshot.char_count();
        let last_line = snapshot.line_count().saturating_sub(1);
        let last_column = snapshot.line_length(last_line).unwrap_or(0);
        self.cursor.char_pos = total_chars;
        self.cursor.line = last_line;
        self.cursor.column = last_column;
        self.extended.clear_preferred_column();
        Ok(true)
    }
}

impl Default for NavigationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_movement() {
        let mut nav = NavigationSystem::new();
        let text = "Hello\nWorld";
        assert!(nav.navigate(text, NavigationAction::MoveCharForward).unwrap());
        assert_eq!(nav.cursor().char_pos, 1);
        assert!(nav.navigate(text, NavigationAction::MoveCharBackward).unwrap());
        assert_eq!(nav.cursor().char_pos, 0);
    }

    #[test]
    fn line_navigation() {
        let mut nav = NavigationSystem::new();
        let text = "Short\nLonger line";
        // Move to second line, column 5
        assert!(nav.navigate(text, NavigationAction::MoveLineDown).unwrap());
        assert!(nav.navigate(text, NavigationAction::MoveLineEnd).unwrap());
        assert_eq!(nav.cursor().line, 1);
        assert_eq!(nav.cursor().column, "Longer line".chars().count());
    }

    #[test]
    fn buffer_bounds() {
        let mut nav = NavigationSystem::new();
        let text = "Only one line";
        assert!(matches!(nav.navigate(text, NavigationAction::MoveCharBackward), Err(NavigationError::StartOfBuffer)));
        assert!(nav.navigate(text, NavigationAction::MoveBufferEnd).unwrap());
        assert!(matches!(nav.navigate(text, NavigationAction::MoveLineDown), Err(NavigationError::EndOfBuffer)));
    }

    #[test]
    fn recover_from_invalid_position() {
        let mut nav = NavigationSystem::new();
        nav.set_cursor(CursorPosition::at(100, 50, 10));
        nav.recover_from_invalid_position("abc").unwrap();
        assert_eq!(nav.cursor().char_pos, 3);
        assert_eq!(nav.cursor().line, 0);
    }

    #[test]
    fn tab_width_calculation() {
        let snapshot = TextSnapshot::with_tab_width("hello\tworld", 4);
        assert_eq!(snapshot.display_width_at(5), 5); // before tab
        assert_eq!(snapshot.display_width_at(6), 8); // after tab (5 + 3 to next tab stop)

        // Test char_pos_for_display_column
        assert_eq!(snapshot.char_pos_for_display_column(0, 5), Some(5)); // before tab
        assert_eq!(snapshot.char_pos_for_display_column(0, 8), Some(6)); // after tab
    }

    #[test]
    fn navigate_with_custom_tab_width() {
        let mut nav = NavigationSystem::new();
        let text = "a\tb";

        // Test with tab width 8
        assert!(nav.navigate_with_tab_width(text, NavigationAction::MoveCharForward, 8).unwrap());
        assert_eq!(nav.cursor().char_pos, 1); // at tab character
        assert!(nav.navigate_with_tab_width(text, NavigationAction::MoveCharForward, 8).unwrap());
        assert_eq!(nav.cursor().char_pos, 2); // at 'b'
    }
}
