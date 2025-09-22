//! ミニバッファUI統合
//!
//! ratatuiを使用したミニバッファの描画とレイアウト統合

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    style::{Color, Modifier, Style},
};
use super::{MinibufferState, MinibufferMode};

/// ミニバッファのUI設定
#[derive(Debug, Clone)]
pub struct MinibufferUIConfig {
    /// ボーダーの表示
    pub show_border: bool,
    /// 補完候補リストの最大高さ
    pub completion_max_height: u16,
    /// 補完候補の表示数制限
    pub completion_limit: usize,
    /// エラーメッセージのスタイル
    pub error_style: Style,
    /// 情報メッセージのスタイル
    pub info_style: Style,
    /// プロンプトのスタイル
    pub prompt_style: Style,
    /// 入力テキストのスタイル
    pub input_style: Style,
    /// 選択された補完候補のスタイル
    pub selected_completion_style: Style,
    /// 通常の補完候補のスタイル
    pub completion_style: Style,
}

impl Default for MinibufferUIConfig {
    fn default() -> Self {
        Self {
            show_border: true,
            completion_max_height: 10,
            completion_limit: 50,
            error_style: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            info_style: Style::default()
                .fg(Color::Green),
            prompt_style: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            input_style: Style::default()
                .fg(Color::White),
            selected_completion_style: Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            completion_style: Style::default()
                .fg(Color::Gray),
        }
    }
}

/// ミニバッファレンダラー
pub struct MinibufferRenderer {
    config: MinibufferUIConfig,
}

impl MinibufferRenderer {
    /// 新しいレンダラーを作成
    pub fn new() -> Self {
        Self {
            config: MinibufferUIConfig::default(),
        }
    }

    /// 設定付きでレンダラーを作成
    pub fn with_config(config: MinibufferUIConfig) -> Self {
        Self { config }
    }

    /// ミニバッファを描画
    pub fn render(&self, frame: &mut Frame, area: Rect, state: &MinibufferState) {
        match &state.mode {
            MinibufferMode::Inactive => {
                // 非アクティブ時は何も描画しない
            }
            MinibufferMode::ErrorDisplay { message, .. } => {
                self.render_message(frame, area, message, self.config.error_style);
            }
            MinibufferMode::InfoDisplay { message, .. } => {
                self.render_message(frame, area, message, self.config.info_style);
            }
            _ => {
                self.render_input_mode(frame, area, state);
            }
        }
    }

    /// メッセージを描画
    fn render_message(&self, frame: &mut Frame, area: Rect, message: &str, style: Style) {
        let paragraph = Paragraph::new(message)
            .style(style)
            .wrap(Wrap { trim: true });

        let block = if self.config.show_border {
            Block::default().borders(Borders::ALL)
        } else {
            Block::default()
        };

        let widget = paragraph.block(block);
        frame.render_widget(widget, area);
    }

    /// 入力モードを描画
    fn render_input_mode(&self, frame: &mut Frame, area: Rect, state: &MinibufferState) {
        // ミニバッファ用の領域を計算
        let minibuffer_height = 1 + if self.config.show_border { 2 } else { 0 };
        let completion_height = self.calculate_completion_height(state);

        let total_height = minibuffer_height + completion_height;
        let layout_area = Rect {
            x: area.x,
            y: area.y.saturating_sub(total_height.saturating_sub(1)),
            width: area.width,
            height: total_height,
        };

        // 垂直レイアウト（下から上に）
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(completion_height),
                Constraint::Length(minibuffer_height),
            ])
            .split(layout_area);

        // 補完候補リスト
        if completion_height > 0 {
            self.render_completions(frame, chunks[0], state);
        }

        // ミニバッファ本体
        self.render_minibuffer_input(frame, chunks[1], state);
    }

    /// ミニバッファの入力部分を描画
    fn render_minibuffer_input(&self, frame: &mut Frame, area: Rect, state: &MinibufferState) {
        // プロンプトと入力を結合
        let prompt_text = format!("{}{}", state.prompt, state.input);
        let cursor_offset = state.prompt.chars().count() + state.cursor_pos;

        let paragraph = Paragraph::new(prompt_text)
            .style(self.config.input_style);

        let block = if self.config.show_border {
            Block::default()
                .borders(Borders::ALL)
                .title(self.get_mode_title(&state.mode))
        } else {
            Block::default()
        };

        let widget = paragraph.block(block);
        frame.render_widget(widget, area);

        // カーソル位置を計算して設定
        if let Some(cursor_x) = self.calculate_cursor_position(area, cursor_offset) {
            let cursor_y = area.y + if self.config.show_border { 1 } else { 0 };
            frame.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }

    /// 補完候補リストを描画
    fn render_completions(&self, frame: &mut Frame, area: Rect, state: &MinibufferState) {
        if state.completions.is_empty() {
            return;
        }

        let items: Vec<ListItem> = state.completions
            .iter()
            .take(self.config.completion_limit)
            .enumerate()
            .map(|(i, completion)| {
                let style = if Some(i) == state.selected_completion {
                    self.config.selected_completion_style
                } else {
                    self.config.completion_style
                };

                ListItem::new(completion.as_str()).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Completions")
            );

        frame.render_widget(list, area);
    }

    /// 補完候補リストの高さを計算
    fn calculate_completion_height(&self, state: &MinibufferState) -> u16 {
        if state.completions.is_empty() {
            return 0;
        }

        let item_count = state.completions.len().min(self.config.completion_limit);
        let border_height = if self.config.show_border { 2 } else { 0 };

        (item_count as u16 + border_height).min(self.config.completion_max_height)
    }

    /// カーソル位置を計算
    fn calculate_cursor_position(&self, area: Rect, cursor_offset: usize) -> Option<u16> {
        let border_offset = if self.config.show_border { 1 } else { 0 };
        let x = area.x + border_offset + cursor_offset as u16;

        if x < area.x + area.width.saturating_sub(border_offset) {
            Some(x)
        } else {
            None // カーソルが表示領域外
        }
    }

    /// モードに応じたタイトルを取得
    fn get_mode_title(&self, mode: &MinibufferMode) -> &str {
        match mode {
            MinibufferMode::FindFile => "Find File",
            MinibufferMode::ExecuteCommand => "M-x",
            MinibufferMode::SaveConfirmation => "Save",
            _ => "Minibuffer",
        }
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: MinibufferUIConfig) {
        self.config = config;
    }

    /// 現在の設定を取得
    pub fn config(&self) -> &MinibufferUIConfig {
        &self.config
    }
}

impl Default for MinibufferRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// ミニバッファレイアウト計算
pub struct MinibufferLayout;

impl MinibufferLayout {
    /// メイン画面のレイアウトを計算（ミニバッファ用の領域を確保）
    pub fn calculate_main_layout(terminal_area: Rect, minibuffer_active: bool) -> (Rect, Rect) {
        if minibuffer_active {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),      // メイン領域
                    Constraint::Length(3),   // ミニバッファ領域
                ])
                .split(terminal_area);

            (chunks[0], chunks[1])
        } else {
            // ミニバッファが非アクティブの場合は全領域をメインに
            (terminal_area, Rect::default())
        }
    }

    /// ミニバッファ用の推奨領域サイズを計算
    pub fn calculate_minibuffer_area(
        terminal_area: Rect,
        completion_count: usize,
        show_border: bool,
    ) -> Rect {
        let border_height = if show_border { 2 } else { 0 };
        let minibuffer_height = 1 + border_height;
        let completion_height = if completion_count > 0 {
            (completion_count as u16).min(10) + border_height
        } else {
            0
        };

        let total_height = minibuffer_height + completion_height;

        Rect {
            x: terminal_area.x,
            y: terminal_area.height.saturating_sub(total_height),
            width: terminal_area.width,
            height: total_height,
        }
    }

    /// ポップアップ用の中央領域を計算
    pub fn calculate_popup_area(terminal_area: Rect, popup_width: u16, popup_height: u16) -> Rect {
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((terminal_area.height.saturating_sub(popup_height)) / 2),
                Constraint::Length(popup_height),
                Constraint::Min(0),
            ])
            .split(terminal_area)[1];

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((terminal_area.width.saturating_sub(popup_width)) / 2),
                Constraint::Length(popup_width),
                Constraint::Min(0),
            ])
            .split(popup_area)[1]
    }
}

/// ミニバッファのスタイルヘルパー
pub struct MinibufferStyles;

impl MinibufferStyles {
    /// デフォルトのスタイルセットを取得
    pub fn default_styles() -> MinibufferUIConfig {
        MinibufferUIConfig::default()
    }

    /// ダークテーマのスタイルセット
    pub fn dark_theme() -> MinibufferUIConfig {
        MinibufferUIConfig {
            show_border: true,
            completion_max_height: 10,
            completion_limit: 50,
            error_style: Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
            info_style: Style::default()
                .fg(Color::LightGreen),
            prompt_style: Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
            input_style: Style::default()
                .fg(Color::White),
            selected_completion_style: Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            completion_style: Style::default()
                .fg(Color::DarkGray),
        }
    }

    /// ライトテーマのスタイルセット
    pub fn light_theme() -> MinibufferUIConfig {
        MinibufferUIConfig {
            show_border: true,
            completion_max_height: 10,
            completion_limit: 50,
            error_style: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            info_style: Style::default()
                .fg(Color::Green),
            prompt_style: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            input_style: Style::default()
                .fg(Color::Black),
            selected_completion_style: Style::default()
                .bg(Color::LightBlue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            completion_style: Style::default()
                .fg(Color::Gray),
        }
    }

    /// カスタムカラーパレット
    pub fn custom_theme(
        primary_color: Color,
        secondary_color: Color,
        background_color: Color,
        text_color: Color,
    ) -> MinibufferUIConfig {
        MinibufferUIConfig {
            show_border: true,
            completion_max_height: 10,
            completion_limit: 50,
            error_style: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            info_style: Style::default()
                .fg(Color::Green),
            prompt_style: Style::default()
                .fg(primary_color)
                .add_modifier(Modifier::BOLD),
            input_style: Style::default()
                .fg(text_color),
            selected_completion_style: Style::default()
                .bg(primary_color)
                .fg(background_color)
                .add_modifier(Modifier::BOLD),
            completion_style: Style::default()
                .fg(secondary_color),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minibuffer_renderer_creation() {
        let renderer = MinibufferRenderer::new();
        assert!(renderer.config().show_border);
        assert_eq!(renderer.config().completion_max_height, 10);
    }

    #[test]
    fn test_custom_config() {
        let mut config = MinibufferUIConfig::default();
        config.show_border = false;
        config.completion_max_height = 5;

        let renderer = MinibufferRenderer::with_config(config);
        assert!(!renderer.config().show_border);
        assert_eq!(renderer.config().completion_max_height, 5);
    }

    #[test]
    fn test_completion_height_calculation() {
        let renderer = MinibufferRenderer::new();
        let mut state = MinibufferState::default();

        // 補完候補なしの場合
        assert_eq!(renderer.calculate_completion_height(&state), 0);

        // 補完候補ありの場合
        state.completions = vec!["test1".to_string(), "test2".to_string()];
        let height = renderer.calculate_completion_height(&state);
        assert!(height > 0);
        assert!(height <= renderer.config().completion_max_height);
    }

    #[test]
    fn test_layout_calculation() {
        let terminal_area = Rect::new(0, 0, 80, 24);

        // アクティブな場合
        let (main_area, minibuffer_area) = MinibufferLayout::calculate_main_layout(terminal_area, true);
        assert_eq!(main_area.height + minibuffer_area.height, terminal_area.height);
        assert_eq!(minibuffer_area.height, 3);

        // 非アクティブな場合
        let (main_area, minibuffer_area) = MinibufferLayout::calculate_main_layout(terminal_area, false);
        assert_eq!(main_area, terminal_area);
        assert_eq!(minibuffer_area, Rect::default());
    }

    #[test]
    fn test_popup_area_calculation() {
        let terminal_area = Rect::new(0, 0, 80, 24);
        let popup_area = MinibufferLayout::calculate_popup_area(terminal_area, 40, 12);

        assert_eq!(popup_area.width, 40);
        assert_eq!(popup_area.height, 12);
        // 中央配置されているかチェック
        assert!(popup_area.x > 0 && popup_area.x < terminal_area.width - popup_area.width);
        assert!(popup_area.y > 0 && popup_area.y < terminal_area.height - popup_area.height);
    }

    #[test]
    fn test_cursor_position_calculation() {
        let renderer = MinibufferRenderer::new();
        let area = Rect::new(0, 0, 80, 3);

        // ボーダーありの場合
        let cursor_pos = renderer.calculate_cursor_position(area, 10);
        assert_eq!(cursor_pos, Some(11)); // border offset + cursor offset

        // ボーダーなしの場合
        let mut config = MinibufferUIConfig::default();
        config.show_border = false;
        let renderer = MinibufferRenderer::with_config(config);
        let cursor_pos = renderer.calculate_cursor_position(area, 10);
        assert_eq!(cursor_pos, Some(10));
    }

    #[test]
    fn test_style_themes() {
        let dark_theme = MinibufferStyles::dark_theme();
        let light_theme = MinibufferStyles::light_theme();
        let default_theme = MinibufferStyles::default_styles();

        // すべてのテーマが有効な設定を持つことを確認
        assert!(dark_theme.completion_max_height > 0);
        assert!(light_theme.completion_max_height > 0);
        assert!(default_theme.completion_max_height > 0);
    }

    #[test]
    fn test_mode_title() {
        let renderer = MinibufferRenderer::new();

        assert_eq!(renderer.get_mode_title(&MinibufferMode::FindFile), "Find File");
        assert_eq!(renderer.get_mode_title(&MinibufferMode::ExecuteCommand), "M-x");
        assert_eq!(renderer.get_mode_title(&MinibufferMode::SaveConfirmation), "Save");
    }
}