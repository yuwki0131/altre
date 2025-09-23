//! カーソル位置管理
//!
//! テキストバッファ内でのカーソル位置を管理

/// カーソル位置を表現する構造体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    /// 文字位置（0ベース）
    pub char_pos: usize,
    /// 行番号（0ベース）
    pub line: usize,
    /// 列番号（0ベース、文字単位）
    pub column: usize,
}

impl CursorPosition {
    /// 新しいカーソル位置を作成（原点に配置）
    pub fn new() -> Self {
        Self {
            char_pos: 0,
            line: 0,
            column: 0,
        }
    }

    /// 指定された位置にカーソルを作成
    pub fn at(char_pos: usize, line: usize, column: usize) -> Self {
        Self {
            char_pos,
            line,
            column,
        }
    }

    /// カーソルを原点に移動
    pub fn move_to_origin(&mut self) {
        self.char_pos = 0;
        self.line = 0;
        self.column = 0;
    }

    /// カーソルを指定位置に移動
    pub fn move_to(&mut self, char_pos: usize, line: usize, column: usize) {
        self.char_pos = char_pos;
        self.line = line;
        self.column = column;
    }

    /// カーソルを相対的に移動
    pub fn move_by(&mut self, char_delta: isize, line_delta: isize, column_delta: isize) {
        self.char_pos = (self.char_pos as isize + char_delta).max(0) as usize;
        self.line = (self.line as isize + line_delta).max(0) as usize;
        self.column = (self.column as isize + column_delta).max(0) as usize;
    }

    /// 前の文字に移動
    pub fn move_backward(&mut self) {
        if self.char_pos > 0 {
            self.char_pos -= 1;
            if self.column > 0 {
                self.column -= 1;
            } else if self.line > 0 {
                // 行の先頭から前の行の末尾に移動
                // TODO: 実際の行長を取得して正確な列位置を設定
                self.line -= 1;
                self.column = 0; // プレースホルダー
            }
        }
    }

    /// 次の文字に移動
    pub fn move_forward(&mut self) {
        self.char_pos += 1;
        self.column += 1;
        // TODO: 改行文字を検出して行・列を更新
    }

    /// 前の行に移動
    pub fn move_up(&mut self) {
        if self.line > 0 {
            self.line -= 1;
            // TODO: バッファから実際の文字位置を計算
        }
    }

    /// 次の行に移動
    pub fn move_down(&mut self) {
        self.line += 1;
        // TODO: バッファから実際の文字位置を計算
    }

    /// 行の先頭に移動
    pub fn move_to_line_start(&mut self) {
        self.column = 0;
        // TODO: バッファから実際の文字位置を計算
    }

    /// 行の末尾に移動
    pub fn move_to_line_end(&mut self) {
        // TODO: バッファから行長を取得して列位置を設定
    }
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self::new()
    }
}

/// カーソル操作の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMovement {
    /// 文字単位の移動
    Forward,
    Backward,
    /// 行単位の移動
    Up,
    Down,
    /// 行内の移動
    LineStart,
    LineEnd,
    /// バッファ全体の移動
    BufferStart,
    BufferEnd,
}

/// カーソル移動ユーティリティ
pub struct CursorMover;

impl CursorMover {
    /// テキスト内容を考慮してカーソル位置を更新
    ///
    /// # Arguments
    /// * `cursor` - 更新するカーソル位置
    /// * `text` - 参照するテキスト内容
    /// * `movement` - 移動の種類
    pub fn move_cursor(
        cursor: &mut CursorPosition,
        text: &str,
        movement: CursorMovement,
    ) -> bool {
        match movement {
            CursorMovement::Forward => Self::move_forward(cursor, text),
            CursorMovement::Backward => Self::move_backward(cursor, text),
            CursorMovement::Up => Self::move_up(cursor, text),
            CursorMovement::Down => Self::move_down(cursor, text),
            CursorMovement::LineStart => Self::move_to_line_start(cursor, text),
            CursorMovement::LineEnd => Self::move_to_line_end(cursor, text),
            CursorMovement::BufferStart => Self::move_to_buffer_start(cursor),
            CursorMovement::BufferEnd => Self::move_to_buffer_end(cursor, text),
        }
    }

    fn move_forward(cursor: &mut CursorPosition, text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        if cursor.char_pos < chars.len() {
            let ch = chars[cursor.char_pos];
            cursor.char_pos += 1;

            if ch == '\n' {
                cursor.line += 1;
                cursor.column = 0;
            } else {
                cursor.column += 1;
            }
            true
        } else {
            false
        }
    }

    fn move_backward(cursor: &mut CursorPosition, text: &str) -> bool {
        if cursor.char_pos > 0 {
            cursor.char_pos -= 1;
            let chars: Vec<char> = text.chars().collect();
            let ch = chars[cursor.char_pos];

            if ch == '\n' {
                cursor.line -= 1;
                // 前の行の長さを計算
                cursor.column = Self::calculate_line_length(text, cursor.line);
            } else {
                cursor.column -= 1;
            }
            true
        } else {
            false
        }
    }

    fn move_up(cursor: &mut CursorPosition, text: &str) -> bool {
        if cursor.line > 0 {
            let target_line = cursor.line - 1;
            let target_column = cursor.column;

            cursor.line = target_line;
            cursor.column = std::cmp::min(
                target_column,
                Self::calculate_line_length(text, target_line),
            );
            cursor.char_pos = Self::calculate_char_position(text, cursor.line, cursor.column);
            true
        } else {
            false
        }
    }

    fn move_down(cursor: &mut CursorPosition, text: &str) -> bool {
        let total_lines = Self::count_lines(text);
        if cursor.line < total_lines.saturating_sub(1) {
            let target_line = cursor.line + 1;
            let target_column = cursor.column;

            cursor.line = target_line;
            cursor.column = std::cmp::min(
                target_column,
                Self::calculate_line_length(text, target_line),
            );
            cursor.char_pos = Self::calculate_char_position(text, cursor.line, cursor.column);
            true
        } else {
            false
        }
    }

    fn move_to_line_start(cursor: &mut CursorPosition, text: &str) -> bool {
        cursor.column = 0;
        cursor.char_pos = Self::calculate_char_position(text, cursor.line, 0);
        true
    }

    fn move_to_line_end(cursor: &mut CursorPosition, text: &str) -> bool {
        let line_length = Self::calculate_line_length(text, cursor.line);
        cursor.column = line_length;
        cursor.char_pos = Self::calculate_char_position(text, cursor.line, line_length);
        true
    }

    fn move_to_buffer_start(cursor: &mut CursorPosition) -> bool {
        cursor.char_pos = 0;
        cursor.line = 0;
        cursor.column = 0;
        true
    }

    fn move_to_buffer_end(cursor: &mut CursorPosition, text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        cursor.char_pos = chars.len();

        let lines: Vec<&str> = text.lines().collect();
        cursor.line = lines.len().saturating_sub(1);
        cursor.column = if lines.is_empty() {
            0
        } else {
            lines[cursor.line].chars().count()
        };
        true
    }

    /// 指定行の文字数を計算
    fn calculate_line_length(text: &str, line: usize) -> usize {
        text.lines()
            .nth(line)
            .map(|l| l.chars().count())
            .unwrap_or(0)
    }

    /// 行数を計算
    fn count_lines(text: &str) -> usize {
        if text.is_empty() {
            1
        } else {
            text.lines().count()
        }
    }

    /// 行・列位置から文字位置を計算
    fn calculate_char_position(text: &str, line: usize, column: usize) -> usize {
        let lines: Vec<&str> = text.lines().collect();
        let mut char_pos = 0;

        for (i, text_line) in lines.iter().enumerate() {
            if i == line {
                char_pos += std::cmp::min(column, text_line.chars().count());
                break;
            } else {
                char_pos += text_line.chars().count() + 1; // +1 for newline
            }
        }

        char_pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_creation() {
        let cursor = CursorPosition::new();
        assert_eq!(cursor.char_pos, 0);
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let text = "Hello\nWorld\n";
        let mut cursor = CursorPosition::new();

        // Forward movement
        assert!(CursorMover::move_cursor(&mut cursor, text, CursorMovement::Forward));
        assert_eq!(cursor.char_pos, 1);
        assert_eq!(cursor.column, 1);

        // Move to newline
        cursor.char_pos = 5;
        cursor.column = 5;
        assert!(CursorMover::move_cursor(&mut cursor, text, CursorMovement::Forward));
        assert_eq!(cursor.line, 1);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_line_navigation() {
        let text = "First line\nSecond line\nThird line";
        let mut cursor = CursorPosition::at(6, 0, 6); // "First " の後

        // Move down
        assert!(CursorMover::move_cursor(&mut cursor, text, CursorMovement::Down));
        assert_eq!(cursor.line, 1);
        assert_eq!(cursor.column, 6);

        // Move up
        assert!(CursorMover::move_cursor(&mut cursor, text, CursorMovement::Up));
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 6);
    }
}