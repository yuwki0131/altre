//! テキストエリア描画
//!
//! メインテキスト編集エリアの描画機能

use ratatui::{
    widgets::{Block, Borders, Paragraph},
    layout::Rect,
    text::{Line, Span},
    style::{Color, Style},
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

    /// テキストコンテンツを行に分割し、カーソル位置を強調
    fn prepare_lines<'a>(&self, content: &'a str) -> Vec<Line<'a>> {
        let text_lines: Vec<&str> = content.lines().collect();
        let mut lines = Vec::new();

        for (line_idx, &line_text) in text_lines.iter().enumerate() {
            if line_idx == self.cursor_line {
                // カーソルがある行は特別処理
                lines.push(self.create_cursor_line(line_text));
            } else {
                // 通常の行
                lines.push(Line::from(line_text));
            }
        }

        // 空のファイルやカーソルが最終行を超えている場合の処理
        if text_lines.is_empty() || self.cursor_line >= text_lines.len() {
            lines.push(self.create_cursor_line(""));
        }

        lines
    }

    /// カーソル位置を含む行を作成
    fn create_cursor_line<'a>(&self, line_text: &'a str) -> Line<'a> {
        let chars: Vec<char> = line_text.chars().collect();
        let mut spans = Vec::new();

        if self.cursor_column == 0 {
            // 行の先頭にカーソル
            spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
            spans.push(Span::raw(line_text));
        } else if self.cursor_column >= chars.len() {
            // 行の末尾またはそれを超えた位置にカーソル
            spans.push(Span::raw(line_text));
            spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
        } else {
            // 行の中央にカーソル
            let before: String = chars[..self.cursor_column].iter().collect();
            let cursor_char = chars[self.cursor_column];
            let after: String = chars[self.cursor_column + 1..].iter().collect();

            spans.push(Span::raw(before));
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black)
            ));
            spans.push(Span::raw(after));
        }

        Line::from(spans)
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
    ) {
        // 簡単な実装：テキストエリア全体に文字列を表示
        let content = editor.to_string();
        let lines: Vec<Line<'_>> = content.lines()
            .map(|line| Line::from(line))
            .collect();

        let paragraph = Paragraph::new(lines)
            .style(theme.style(&ComponentType::TextArea));

        frame.render_widget(paragraph, area);
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