//! テキストエリア描画
//!
//! メインテキスト編集エリアの描画機能

use ratatui::{
    widgets::{Block, Borders, Paragraph},
    layout::Rect,
    text::Line,
    Frame,
};
use crate::buffer::TextEditor;
use crate::ui::theme::{Theme, ComponentType};

/// ビューポート情報
#[derive(Debug, Clone)]
pub struct Viewport {
    pub start_line: usize,
    pub end_line: usize,
    pub scroll_x: usize,
}

/// テキストエリア描画器
#[derive(Debug)]
pub struct TextArea {
    /// 境界線を表示するか
    show_border: bool,
    /// カーソル位置
    cursor_line: usize,
    cursor_column: usize,
}

impl TextArea {
    /// 新しいテキストエリアを作成
    pub fn new() -> Self {
        Self {
            show_border: false,
            cursor_line: 0,
            cursor_column: 0,
        }
    }

    /// 境界線表示を設定
    pub fn with_border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    /// カーソル位置を設定
    pub fn set_cursor(&mut self, line: usize, column: usize) {
        self.cursor_line = line;
        self.cursor_column = column;
    }

    /// テキストを描画
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, content: &str) {
        let lines = self.prepare_lines(content);

        let mut paragraph = Paragraph::new(lines);

        if self.show_border {
            paragraph = paragraph.block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("altre")
            );
        }

        frame.render_widget(paragraph, area);

        // カーソル描画（将来実装）
        // self.render_cursor(frame, area);
    }

    /// テキストコンテンツを行に分割（カーソルは別途描画）
    pub fn prepare_lines<'a>(&self, content: &'a str) -> Vec<Line<'a>> {
        let text_lines: Vec<&str> = content.lines().collect();
        let mut lines = Vec::new();

        for &line_text in text_lines.iter() {
            // 全て通常の行として処理（カーソルは別途描画）
            lines.push(Line::from(line_text));
        }

        // 空のファイルの場合は空行を追加
        if text_lines.is_empty() {
            lines.push(Line::from(""));
        }

        lines
    }

    /// 画面上のカーソル位置を計算
    pub fn calculate_cursor_screen_position(&self, area: Rect, viewport_start_line: usize) -> Option<(u16, u16)> {
        // カーソル行が表示領域内にあるかチェック
        if self.cursor_line < viewport_start_line {
            return None;
        }

        let screen_line = self.cursor_line - viewport_start_line;
        if screen_line >= area.height as usize {
            return None;
        }

        // 境界線がある場合は内側に調整
        let content_x = if self.show_border { area.x + 1 } else { area.x };
        let content_y = if self.show_border { area.y + 1 } else { area.y };

        let cursor_x = content_x + self.cursor_column as u16;
        let cursor_y = content_y + screen_line as u16;

        // 表示領域内かチェック
        let max_x = if self.show_border { area.x + area.width - 1 } else { area.x + area.width };
        let max_y = if self.show_border { area.y + area.height - 1 } else { area.y + area.height };

        if cursor_x < max_x && cursor_y < max_y {
            Some((cursor_x, cursor_y))
        } else {
            None
        }
    }

    /// 行数を取得
    pub fn line_count(&self, content: &str) -> usize {
        if content.is_empty() {
            1
        } else {
            content.lines().count()
        }
    }

    /// 指定行の文字数を取得
    pub fn line_length(&self, content: &str, line_idx: usize) -> usize {
        content
            .lines()
            .nth(line_idx)
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    /// テキストエリアのサイズに基づいて表示範囲を計算
    pub fn calculate_visible_range(&self, area: Rect, total_lines: usize) -> (usize, usize) {
        let visible_lines = area.height as usize;
        let start_line = if self.cursor_line < visible_lines {
            0
        } else {
            self.cursor_line.saturating_sub(visible_lines / 2)
        };

        let end_line = std::cmp::min(start_line + visible_lines, total_lines);
        (start_line, end_line)
    }
}

/// 高性能テキストエリアレンダラー
#[derive(Debug)]
pub struct TextAreaRenderer {
    /// 行番号表示
    #[allow(dead_code)]
    show_line_numbers: bool,
}

impl TextAreaRenderer {
    /// 新しいレンダラーを作成
    pub fn new() -> Self {
        Self {
            show_line_numbers: true,
        }
    }

    /// テキストエリアを描画
    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        editor: &TextEditor,
        theme: &Theme,
    ) -> Option<(u16, u16)> {
        let content = editor.to_string();
        let cursor_pos = editor.cursor();

        // TextAreaを使ってテキスト描画
        let mut text_area = TextArea::new();
        text_area.set_cursor(cursor_pos.line, cursor_pos.column);

        let lines = text_area.prepare_lines(&content);

        let paragraph = Paragraph::new(lines)
            .style(theme.style(&ComponentType::TextArea));

        frame.render_widget(paragraph, area);

        // カーソル位置を計算して返す（ビューポートオフセットは0とする）
        text_area.calculate_cursor_screen_position(area, 0)
    }
}

impl Default for TextAreaRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_area_creation() {
        let text_area = TextArea::new();
        assert!(!text_area.show_border);
        assert_eq!(text_area.cursor_line, 0);
        assert_eq!(text_area.cursor_column, 0);
    }

    #[test]
    fn test_cursor_setting() {
        let mut text_area = TextArea::new();
        text_area.set_cursor(5, 10);
        assert_eq!(text_area.cursor_line, 5);
        assert_eq!(text_area.cursor_column, 10);
    }

    #[test]
    fn test_line_count() {
        let text_area = TextArea::new();
        assert_eq!(text_area.line_count(""), 1);
        assert_eq!(text_area.line_count("Hello\nWorld"), 2);
        assert_eq!(text_area.line_count("Single line"), 1);
    }

    #[test]
    fn test_line_length() {
        let text_area = TextArea::new();
        let content = "Hello\nWorld\nこんにちは";

        assert_eq!(text_area.line_length(content, 0), 5); // "Hello"
        assert_eq!(text_area.line_length(content, 1), 5); // "World"
        assert_eq!(text_area.line_length(content, 2), 5); // "こんにちは"
        assert_eq!(text_area.line_length(content, 3), 0); // 存在しない行
    }

    #[test]
    fn test_visible_range_calculation() {
        let text_area = TextArea::new();
        let area = Rect::new(0, 0, 80, 20);

        // カーソルが上部にある場合
        let (start, end) = text_area.calculate_visible_range(area, 50);
        assert_eq!(start, 0);
        assert_eq!(end, 20);

        // カーソルが中央付近にある場合
        let mut text_area = TextArea::new();
        text_area.set_cursor(30, 0);
        let (start, end) = text_area.calculate_visible_range(area, 50);
        assert_eq!(start, 20); // 30 - 20/2
        assert_eq!(end, 40);  // 20 + 20
    }
}