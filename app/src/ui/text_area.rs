//! テキストエリア描画
//!
//! メインテキスト編集エリアの描画機能

use std::collections::HashMap;

use ratatui::{
    widgets::{Block, Borders, Paragraph},
    layout::Rect,
    text::{Line, Span},
    style::{Color, Modifier, Style},
    Frame,
};
use crate::buffer::TextEditor;
use crate::search::SearchHighlight;
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
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, content: &str, highlights: &[SearchHighlight]) {
        let lines = self.prepare_lines(content, highlights);

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
    pub fn prepare_lines(&self, content: &str, highlights: &[SearchHighlight]) -> Vec<Line<'static>> {
        let text_lines: Vec<&str> = content.lines().collect();
        let mut lines = Vec::new();

        let mut grouped: HashMap<usize, Vec<&SearchHighlight>> = HashMap::new();
        for highlight in highlights {
            grouped.entry(highlight.line).or_default().push(highlight);
        }
        for list in grouped.values_mut() {
            list.sort_by_key(|h| h.start_column);
        }

        if text_lines.is_empty() {
            lines.push(Line::from(""));
            return lines;
        }

        for (idx, &line_text) in text_lines.iter().enumerate() {
            if let Some(highlights) = grouped.get(&idx) {
                lines.push(build_highlighted_line(line_text, highlights));
            } else {
                lines.push(Line::from(line_text.to_string()));
            }
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
        highlights: &[SearchHighlight],
        minibuffer_active: bool,
    ) -> Option<(u16, u16)> {
        let content = editor.to_string();
        let cursor_pos = editor.cursor();

        // TextAreaを使ってテキスト描画
        let mut text_area = TextArea::new();
        text_area.set_cursor(cursor_pos.line, cursor_pos.column);

        // ビューポート管理：ミニバッファがアクティブな時のスクロール調整
        let viewport_offset = if minibuffer_active {
            self.calculate_viewport_offset(cursor_pos.line, area.height as usize)
        } else {
            0
        };

        let all_lines = text_area.prepare_lines(&content, highlights);

        // 表示範囲を計算してスライス
        let visible_lines: Vec<Line> = if viewport_offset < all_lines.len() {
            all_lines[viewport_offset..].iter()
                .take(area.height as usize)
                .cloned()
                .collect()
        } else {
            vec![Line::from("")] // 空行を表示
        };

        let paragraph = Paragraph::new(visible_lines)
            .style(theme.style(&ComponentType::TextArea));

        frame.render_widget(paragraph, area);

        // カーソル位置を計算して返す
        text_area.calculate_cursor_screen_position(area, viewport_offset)
    }

    /// ビューポートオフセットを計算
    fn calculate_viewport_offset(&self, cursor_line: usize, area_height: usize) -> usize {
        // ミニバッファがアクティブな時、カーソル行が見えるようにオフセット調整
        if cursor_line < area_height {
            0  // カーソルが表示領域内にある場合はオフセット不要
        } else {
            // カーソルを中央付近に表示するようオフセット
            cursor_line.saturating_sub(area_height / 2)
        }
    }
}

fn build_highlighted_line(line_text: &str, highlights: &[&SearchHighlight]) -> Line<'static> {
    if highlights.is_empty() {
        return Line::from(line_text.to_string());
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let line_len = line_text.chars().count();
    let mut cursor = 0usize;

    for highlight in highlights {
        if highlight.start_column >= line_len {
            continue;
        }

        let start = highlight.start_column.min(line_len);
        let end = highlight.end_column.min(line_len);

        if start > cursor {
            spans.push(Span::raw(substring_by_char(line_text, cursor, start)));
        }

        if end > start {
            let segment = substring_by_char(line_text, start, end);
            let style = if highlight.is_current {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(0, 80, 80))
            };
            spans.push(Span::styled(segment, style));
        }

        cursor = end;
    }

    if cursor < line_len {
        spans.push(Span::raw(substring_by_char(line_text, cursor, line_len)));
    }

    if spans.is_empty() {
        Line::from(line_text.to_string())
    } else {
        Line::from(spans)
    }
}

fn substring_by_char(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end.saturating_sub(start)).collect()
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
