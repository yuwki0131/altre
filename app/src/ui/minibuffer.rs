//! ミニバッファ描画
//!
//! 画面下部のミニバッファ描画機能

use ratatui::{
    widgets::{Block, Borders, Paragraph},
    layout::Rect,
    text::Line,
    style::{Color, Style},
    Frame,
};

/// ミニバッファの状態
#[derive(Debug, Clone)]
pub enum MinibufferState {
    /// 通常状態（何も表示しない）
    Normal,
    /// プロンプト表示（ユーザー入力待ち）
    Prompt { message: String, input: String },
    /// メッセージ表示
    Message { text: String, is_error: bool },
    /// キーシーケンス表示
    KeySequence { sequence: String },
}

/// ミニバッファ描画器
#[derive(Debug)]
pub struct MinibufferRenderer {
    /// 現在の状態
    state: MinibufferState,
    /// 境界線を表示するか
    show_border: bool,
}

impl MinibufferRenderer {
    /// 新しいミニバッファ描画器を作成
    pub fn new() -> Self {
        Self {
            state: MinibufferState::Normal,
            show_border: false,
        }
    }

    /// 境界線表示を設定
    pub fn with_border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    /// 状態を設定
    pub fn set_state(&mut self, state: MinibufferState) {
        self.state = state;
    }

    /// プロンプト状態にする
    pub fn set_prompt(&mut self, message: String, input: String) {
        self.state = MinibufferState::Prompt { message, input };
    }

    /// メッセージを表示
    pub fn set_message(&mut self, text: String) {
        self.state = MinibufferState::Message { text, is_error: false };
    }

    /// エラーメッセージを表示
    pub fn set_error(&mut self, text: String) {
        self.state = MinibufferState::Message { text, is_error: true };
    }

    /// キーシーケンスを表示
    pub fn set_key_sequence(&mut self, sequence: String) {
        self.state = MinibufferState::KeySequence { sequence };
    }

    /// 通常状態にリセット
    pub fn clear(&mut self) {
        self.state = MinibufferState::Normal;
    }

    /// ミニバッファを描画
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        let content = self.create_content();
        let style = self.get_style();

        let mut paragraph = Paragraph::new(content).style(style);

        if self.show_border {
            paragraph = paragraph.block(Block::default().borders(Borders::ALL));
        }

        frame.render_widget(paragraph, area);
    }

    /// 表示内容を作成
    fn create_content(&self) -> Line<'_> {
        match &self.state {
            MinibufferState::Normal => Line::from(""),
            MinibufferState::Prompt { message, input } => {
                Line::from(format!("{}{}", message, input))
            }
            MinibufferState::Message { text, .. } => {
                Line::from(text.clone())
            }
            MinibufferState::KeySequence { sequence } => {
                Line::from(format!("キー: {}", sequence))
            }
        }
    }

    /// 状態に応じたスタイルを取得
    fn get_style(&self) -> Style {
        match &self.state {
            MinibufferState::Normal => Style::default(),
            MinibufferState::Prompt { .. } => Style::default().fg(Color::Cyan),
            MinibufferState::Message { is_error, .. } => {
                if *is_error {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                }
            }
            MinibufferState::KeySequence { .. } => Style::default().fg(Color::Yellow),
        }
    }

    /// 現在の状態を取得
    pub fn current_state(&self) -> &MinibufferState {
        &self.state
    }

    /// プロンプト状態かどうか
    pub fn is_prompting(&self) -> bool {
        matches!(self.state, MinibufferState::Prompt { .. })
    }

    /// エラー状態かどうか
    pub fn is_error(&self) -> bool {
        matches!(self.state, MinibufferState::Message { is_error: true, .. })
    }

    /// プロンプト入力文字列を取得
    pub fn get_prompt_input(&self) -> Option<&str> {
        match &self.state {
            MinibufferState::Prompt { input, .. } => Some(input),
            _ => None,
        }
    }

    /// プロンプト入力文字列を更新
    pub fn update_prompt_input(&mut self, input: String) {
        if let MinibufferState::Prompt { message, .. } = &self.state {
            let message = message.clone();
            self.state = MinibufferState::Prompt { message, input };
        }
    }

    /// プロンプト入力に文字を追加
    pub fn append_to_prompt(&mut self, ch: char) {
        if let MinibufferState::Prompt { message: _, input } = &mut self.state {
            input.push(ch);
        }
    }

    /// プロンプト入力から最後の文字を削除
    pub fn backspace_prompt(&mut self) -> bool {
        if let MinibufferState::Prompt { input, .. } = &mut self.state {
            if !input.is_empty() {
                input.pop();
                return true;
            }
        }
        false
    }
}

impl Default for MinibufferRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// ミニバッファユーティリティ
pub struct MinibufferUtils;

impl MinibufferUtils {
    /// メッセージの表示時間を計算（文字数に基づく）
    pub fn calculate_display_duration(message: &str) -> std::time::Duration {
        let base_duration = std::time::Duration::from_secs(2);
        let char_count = message.chars().count();
        let additional = std::time::Duration::from_millis(char_count as u64 * 50);
        base_duration + additional
    }

    /// テキストを指定幅に収まるように切り詰め
    pub fn truncate_text(text: &str, max_width: usize) -> String {
        if text.chars().count() <= max_width {
            text.to_string()
        } else {
            let mut result = String::new();
            let mut count = 0;
            for ch in text.chars() {
                if count + 3 >= max_width { // "..." のスペースを確保
                    result.push_str("...");
                    break;
                }
                result.push(ch);
                count += 1;
            }
            result
        }
    }

    /// ファイルパス補完用の表示文字列を生成
    pub fn format_completion_candidates(candidates: &[String], max_width: usize) -> String {
        if candidates.is_empty() {
            return String::new();
        }

        if candidates.len() == 1 {
            return Self::truncate_text(&candidates[0], max_width);
        }

        // 複数候補がある場合は共通プレフィックスを表示
        let common_prefix = Self::find_common_prefix(candidates);
        if !common_prefix.is_empty() {
            format!("{}... ({} candidates)",
                Self::truncate_text(&common_prefix, max_width.saturating_sub(20)),
                candidates.len())
        } else {
            format!("{} candidates", candidates.len())
        }
    }

    /// 共通プレフィックスを見つける
    fn find_common_prefix(strings: &[String]) -> String {
        if strings.is_empty() {
            return String::new();
        }

        let first = &strings[0];
        let mut prefix = String::new();

        for (i, ch) in first.chars().enumerate() {
            if strings.iter().all(|s| s.chars().nth(i) == Some(ch)) {
                prefix.push(ch);
            } else {
                break;
            }
        }

        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minibuffer_creation() {
        let renderer = MinibufferRenderer::new();
        assert!(matches!(renderer.state, MinibufferState::Normal));
        assert!(!renderer.show_border);
    }

    #[test]
    fn test_state_changes() {
        let mut renderer = MinibufferRenderer::new();

        renderer.set_message("Test message".to_string());
        assert!(matches!(renderer.state, MinibufferState::Message { .. }));

        renderer.set_error("Error message".to_string());
        assert!(renderer.is_error());

        renderer.set_prompt("Enter: ".to_string(), "input".to_string());
        assert!(renderer.is_prompting());
        assert_eq!(renderer.get_prompt_input(), Some("input"));

        renderer.clear();
        assert!(matches!(renderer.state, MinibufferState::Normal));
    }

    #[test]
    fn test_prompt_input_manipulation() {
        let mut renderer = MinibufferRenderer::new();
        renderer.set_prompt("Enter: ".to_string(), "test".to_string());

        renderer.append_to_prompt('!');
        assert_eq!(renderer.get_prompt_input(), Some("test!"));

        assert!(renderer.backspace_prompt());
        assert_eq!(renderer.get_prompt_input(), Some("test"));
    }

    #[test]
    fn test_text_truncation() {
        let text = "This is a very long text that should be truncated";
        let truncated = MinibufferUtils::truncate_text(text, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_common_prefix_finding() {
        let strings = vec![
            "prefix_file1.txt".to_string(),
            "prefix_file2.txt".to_string(),
            "prefix_file3.txt".to_string(),
        ];
        let prefix = MinibufferUtils::find_common_prefix(&strings);
        assert_eq!(prefix, "prefix_file");

        let no_common = vec![
            "file1.txt".to_string(),
            "document.txt".to_string(),
        ];
        let no_prefix = MinibufferUtils::find_common_prefix(&no_common);
        assert!(no_prefix.is_empty());
    }

    #[test]
    fn test_completion_formatting() {
        let candidates = vec!["file1.txt".to_string(), "file2.txt".to_string()];
        let formatted = MinibufferUtils::format_completion_candidates(&candidates, 50);
        assert!(formatted.contains("file"));
        assert!(formatted.contains("2 candidates"));
    }
}
