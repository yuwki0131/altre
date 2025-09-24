//! TUIレイアウト管理
//!
//! ratatuiベースの画面レイアウト計算と管理
//! QA回答に基づく設計：60x15最小サイズ、16色対応、日本語基本対応

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::error::{AltreError, UiError};

/// 画面領域の種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AreaType {
    /// テキストエリア
    TextArea,
    /// ステータスライン
    StatusLine,
    /// ミニバッファ
    Minibuffer,
    /// 行番号
    LineNumbers,
}

/// アプリケーション全体のレイアウト（QA回答反映）
#[derive(Debug, Clone)]
pub struct AppLayout {
    /// ミニバッファエリア（上部、3行）
    pub minibuffer: Rect,
    /// メインエディタエリア（中央、可変）
    pub editor: Rect,
    /// モードラインエリア（下部、1行）
    pub modeline: Rect,
    /// 全体エリア
    pub total: Rect,
}

/// レイアウトマネージャー
#[derive(Debug)]
pub struct LayoutManager {
    /// 最小必要サイズ（QA Q13: 60x15）
    min_width: u16,
    min_height: u16,
    /// 現在の画面サイズ
    current_size: (u16, u16),
    /// レンダリング状態
    render_state: RenderState,
}

/// レンダリング状態管理
#[derive(Debug)]
pub struct RenderState {
    last_frame_time: Option<Instant>,
    dirty_regions: Vec<Rect>,
    frame_count: u64,
}

/// 色設定（QA Q14: 16色対応）
#[derive(Debug, Clone)]
pub struct ColorScheme {
    // 基本色
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,

    // UI要素色
    pub line_number: Color,
    pub line_number_current: Color,
    pub mode_line_bg: Color,
    pub mode_line_fg: Color,

    // ミニバッファ色
    pub minibuffer_prompt: Color,
    pub minibuffer_input: Color,
    pub minibuffer_completion: Color,
    pub minibuffer_selected: Color,

    // メッセージ色
    pub error_message: Color,
    pub warning_message: Color,
    pub info_message: Color,

    // ボーダー色
    pub border_normal: Color,
    pub border_focus: Color,
}

impl LayoutManager {
    /// 新しいレイアウトマネージャーを作成（QA回答反映）
    pub fn new() -> Self {
        Self {
            min_width: 60,   // QA Q13の回答
            min_height: 15,  // QA Q13の回答
            current_size: (0, 0),
            render_state: RenderState::new(),
        }
    }

    /// 画面サイズを更新
    pub fn update_size(&mut self, width: u16, height: u16) -> Result<(), AltreError> {
        if width < self.min_width || height < self.min_height {
            return Err(AltreError::Ui(UiError::ScreenTooSmall { width, height }));
        }

        self.current_size = (width, height);
        self.render_state.mark_layout_dirty();
        Ok(())
    }

    /// 画面サイズからレイアウトを計算（新設計）
    pub fn calculate_layout(&self, area: Rect) -> Result<AppLayout, AltreError> {
        if area.width < self.min_width || area.height < self.min_height {
            return Err(AltreError::Ui(UiError::ScreenTooSmall {
                width: area.width,
                height: area.height
            }));
        }

        // ミニバッファの高さを動的に決定
        let minibuffer_height = 1;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(minibuffer_height), // ミニバッファ（上部）
                Constraint::Min(1),                    // エディタ（中央）
                Constraint::Length(1),                 // モードライン（下部）
            ])
            .split(area);

        Ok(AppLayout {
            minibuffer: chunks[0],
            editor: chunks[1],
            modeline: chunks[2],
            total: area,
        })
    }

    /// 高性能レンダラー用のレイアウト計算
    pub fn calculate_areas(
        &self,
        area: Rect,
        minibuffer_active: bool,
        show_status_line: bool,
    ) -> HashMap<AreaType, Rect> {
        let mut areas = HashMap::new();

        if area.width < self.min_width || area.height < self.min_height {
            // 最小サイズを満たさない場合はデフォルト値を返す
            areas.insert(AreaType::TextArea, area);
            return areas;
        }

        let mut constraints = Vec::new();
        let mut area_order = Vec::new();

        // レイアウト順序：ミニバッファ（上部）→テキストエリア→ステータスライン

        // ミニバッファ（上部、アクティブな場合のみ）
        if minibuffer_active {
            constraints.push(Constraint::Length(1));
            area_order.push(AreaType::Minibuffer);
        }

        // メインのテキストエリア（中央）
        constraints.push(Constraint::Min(1));
        area_order.push(AreaType::TextArea);

        // ステータスライン（下部、表示する場合）
        if show_status_line {
            constraints.push(Constraint::Length(1));
            area_order.push(AreaType::StatusLine);
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // 計算された領域をHashMapに格納
        for (i, area_type) in area_order.iter().enumerate() {
            if i < chunks.len() {
                areas.insert(area_type.clone(), chunks[i]);
            }
        }

        areas
    }

    /// 最小サイズをチェック
    pub fn check_minimum_size(&self, area: Rect) -> bool {
        area.width >= self.min_width && area.height >= self.min_height
    }

    /// 最小サイズ要件を取得
    pub fn minimum_size(&self) -> (u16, u16) {
        (self.min_width, self.min_height)
    }

    /// レンダリング状態を取得
    pub fn render_state(&self) -> &RenderState {
        &self.render_state
    }

    /// レンダリング状態を更新
    pub fn render_state_mut(&mut self) -> &mut RenderState {
        &mut self.render_state
    }

    /// フレーム開始処理
    pub fn begin_frame(&mut self) {
        self.render_state.begin_frame();
    }

    /// フレーム終了処理
    pub fn end_frame(&mut self) {
        self.render_state.end_frame();
    }
}

impl RenderState {
    pub fn new() -> Self {
        Self {
            last_frame_time: None,
            dirty_regions: Vec::new(),
            frame_count: 0,
        }
    }

    pub fn mark_dirty(&mut self, area: Rect) {
        self.dirty_regions.push(area);
    }

    pub fn mark_layout_dirty(&mut self) {
        self.dirty_regions.clear();
        // レイアウト全体が変更されたことを示すために特別なフラグ
        self.dirty_regions.push(Rect::new(0, 0, u16::MAX, u16::MAX));
    }

    pub fn should_redraw(&self, area: Rect) -> bool {
        if self.dirty_regions.is_empty() {
            return false;
        }

        // 全体が汚れている場合
        if self.dirty_regions.iter().any(|r| r.width == u16::MAX) {
            return true;
        }

        // 指定エリアと重複する汚れた領域があるか
        self.dirty_regions.iter().any(|dirty| dirty.intersects(area))
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_regions.clear();
    }

    pub fn begin_frame(&mut self) {
        self.last_frame_time = Some(Instant::now());
        self.frame_count += 1;
    }

    pub fn end_frame(&mut self) {
        self.clear_dirty();
    }

    pub fn get_frame_stats(&self) -> Option<FrameStats> {
        self.last_frame_time.map(|start| {
            FrameStats {
                frame_time: start.elapsed(),
                frame_number: self.frame_count,
                dirty_regions_count: self.dirty_regions.len(),
            }
        })
    }
}

/// フレーム統計情報
#[derive(Debug, Clone)]
pub struct FrameStats {
    pub frame_time: Duration,
    pub frame_number: u64,
    pub dirty_regions_count: usize,
}

impl ColorScheme {
    /// デフォルト色設定（16色対応）
    pub fn default_16_color() -> Self {
        Self {
            // 基本色
            background: Color::Black,
            foreground: Color::White,
            cursor: Color::Yellow,
            selection: Color::Blue,

            // UI要素色
            line_number: Color::DarkGray,
            line_number_current: Color::White,
            mode_line_bg: Color::DarkGray,
            mode_line_fg: Color::White,

            // ミニバッファ色
            minibuffer_prompt: Color::Cyan,
            minibuffer_input: Color::White,
            minibuffer_completion: Color::Gray,
            minibuffer_selected: Color::Black, // 背景はBlue

            // メッセージ色
            error_message: Color::Red,
            warning_message: Color::Yellow,
            info_message: Color::Green,

            // ボーダー色
            border_normal: Color::DarkGray,
            border_focus: Color::Cyan,
        }
    }

    /// 高コントラスト色設定
    pub fn high_contrast() -> Self {
        let mut scheme = Self::default_16_color();
        scheme.background = Color::Black;
        scheme.foreground = Color::White;
        scheme.line_number = Color::White;
        scheme.cursor = Color::White;
        scheme
    }
}

/// 日本語文字幅計算（QA Q15: 基本レベル）
pub fn char_width(ch: char) -> usize {
    match ch {
        // ASCII文字
        '\x00'..='\x7F' => 1,

        // 制御文字
        '\u{0080}'..='\u{009F}' => 0,

        // 全角文字の基本的な判定
        // ひらがな・カタカナ
        '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => 2,

        // CJK統合漢字
        '\u{4E00}'..='\u{9FFF}' => 2,

        // 全角記号
        '\u{FF01}'..='\u{FF60}' => 2,

        // その他は unicode-width を使用
        _ => {
            unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1)
        }
    }
}

/// 文字列の表示幅計算
pub fn string_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// 指定幅で文字列を切り詰め
pub fn truncate_string(s: &str, max_width: usize) -> String {
    let mut width = 0;
    let mut result = String::new();

    for ch in s.chars() {
        let ch_width = char_width(ch);
        if width + ch_width > max_width {
            break;
        }
        width += ch_width;
        result.push(ch);
    }

    result
}

/// 文字列を指定幅にパディング
pub fn pad_string(s: &str, target_width: usize) -> String {
    let current_width = string_width(s);
    if current_width >= target_width {
        truncate_string(s, target_width)
    } else {
        let padding = " ".repeat(target_width - current_width);
        format!("{}{}", s, padding)
    }
}

/// パフォーマンス測定
pub struct PerformanceMetrics {
    pub target_fps: u32,
    pub max_frame_time: Duration,
    pub current_fps: f64,
    pub average_frame_time: Duration,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            target_fps: 60,
            max_frame_time: Duration::from_millis(16), // 1/60秒
            current_fps: 0.0,
            average_frame_time: Duration::ZERO,
        }
    }

    pub fn update(&mut self, frame_time: Duration) {
        if frame_time > Duration::ZERO {
            self.current_fps = 1.0 / frame_time.as_secs_f64();
        }
        self.average_frame_time = frame_time;
    }

    pub fn is_performance_good(&self) -> bool {
        self.average_frame_time <= self.max_frame_time
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_calculation_new_design() {
        let manager = LayoutManager::new();
        let area = Rect::new(0, 0, 80, 25);
        let layout = manager.calculate_layout(area).unwrap();

        assert_eq!(layout.total, area);
        assert_eq!(layout.minibuffer.height, 1); // ミニバッファは固定1行（上部）
        assert_eq!(layout.modeline.height, 1);   // モードライン（下部）
        assert_eq!(layout.editor.height, 23);    // エディタ（中央）: 25 - 1 - 1

        // ミニバッファが上部に配置されることを確認
        assert_eq!(layout.minibuffer.y, 0);
        assert_eq!(layout.editor.y, 1);
        assert_eq!(layout.modeline.y, 24);
    }

    #[test]
    fn test_layout_calculation_minimum_size() {
        let manager = LayoutManager::new();
        let area = Rect::new(0, 0, 60, 15); // 最小サイズ
        let layout = manager.calculate_layout(area).unwrap();

        assert_eq!(layout.total, area);
        assert_eq!(layout.minibuffer.height, 1); // ミニバッファは固定1行（上部）
        assert_eq!(layout.modeline.height, 1);   // モードライン（下部）
        assert_eq!(layout.editor.height, 13);    // エディタ（中央）: 15 - 1 - 1

        // ミニバッファが上部に配置されることを確認
        assert_eq!(layout.minibuffer.y, 0);
        assert_eq!(layout.editor.y, 1);
        assert_eq!(layout.modeline.y, 14);
    }

    #[test]
    fn test_minimum_size_check_qa_values() {
        let manager = LayoutManager::new();

        // QA Q13の回答: 60x15が最小
        let minimum_area = Rect::new(0, 0, 60, 15);
        assert!(manager.check_minimum_size(minimum_area));

        let large_area = Rect::new(0, 0, 100, 50);
        assert!(manager.check_minimum_size(large_area));

        let too_small_area = Rect::new(0, 0, 59, 14);
        assert!(!manager.check_minimum_size(too_small_area));
    }

    #[test]
    fn test_size_update_error_handling() {
        let mut manager = LayoutManager::new();

        // 正常なサイズ更新
        assert!(manager.update_size(80, 25).is_ok());

        // 小さすぎるサイズ
        let result = manager.update_size(59, 14);
        assert!(result.is_err());
        if let Err(AltreError::Ui(UiError::ScreenTooSmall { width, height })) = result {
            assert_eq!(width, 59);
            assert_eq!(height, 14);
        } else {
            panic!("Expected ScreenTooSmall error");
        }
    }

    #[test]
    fn test_japanese_char_width_qa_basic_level() {
        // ASCII文字
        assert_eq!(char_width('a'), 1);
        assert_eq!(char_width('1'), 1);
        assert_eq!(char_width(' '), 1);

        // 日本語文字（QA Q15: 基本レベル対応）
        assert_eq!(char_width('あ'), 2);
        assert_eq!(char_width('ア'), 2);
        assert_eq!(char_width('漢'), 2);
        assert_eq!(char_width('字'), 2);

        // 全角記号
        assert_eq!(char_width('，'), 2);
        assert_eq!(char_width('．'), 2);
    }

    #[test]
    fn test_string_width_calculation() {
        assert_eq!(string_width("hello"), 5);
        assert_eq!(string_width("こんにちは"), 10);
        assert_eq!(string_width("hello世界"), 9); // 5 + 4
        assert_eq!(string_width(""), 0);
    }

    #[test]
    fn test_string_truncation() {
        assert_eq!(truncate_string("hello world", 5), "hello");
        assert_eq!(truncate_string("こんにちは", 6), "こんに"); // 6文字幅 = 3文字
        assert_eq!(truncate_string("hello世界", 7), "hello世"); // 7文字幅
        assert_eq!(truncate_string("short", 10), "short");
    }

    #[test]
    fn test_string_padding() {
        assert_eq!(pad_string("hello", 10), "hello     ");
        assert_eq!(pad_string("こんにちは", 12), "こんにちは  "); // 10+2=12
        assert_eq!(pad_string("too long string", 5), "too l");
    }

    #[test]
    fn test_color_scheme_16_color() {
        let scheme = ColorScheme::default_16_color();

        // 基本色が設定されている
        assert_eq!(scheme.background, Color::Black);
        assert_eq!(scheme.foreground, Color::White);
        assert_eq!(scheme.cursor, Color::Yellow);

        // エラーメッセージ色が適切
        assert_eq!(scheme.error_message, Color::Red);
        assert_eq!(scheme.warning_message, Color::Yellow);
        assert_eq!(scheme.info_message, Color::Green);
    }

    #[test]
    fn test_render_state_dirty_tracking() {
        let mut state = RenderState::new();

        // 初期状態では何も汚れていない
        let area = Rect::new(0, 0, 10, 10);
        assert!(!state.should_redraw(area));

        // 特定領域を汚す
        state.mark_dirty(Rect::new(5, 5, 5, 5));
        assert!(state.should_redraw(Rect::new(5, 5, 5, 5))); // 同じ領域
        assert!(state.should_redraw(Rect::new(6, 6, 2, 2))); // 重複領域
        assert!(!state.should_redraw(Rect::new(0, 0, 2, 2))); // 無関係な領域

        // クリア
        state.clear_dirty();
        assert!(!state.should_redraw(area));
    }

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();

        // 良好なパフォーマンス
        let good_time = Duration::from_millis(10);
        metrics.update(good_time);
        assert!(metrics.is_performance_good());

        // 悪いパフォーマンス
        let bad_time = Duration::from_millis(30);
        metrics.update(bad_time);
        assert!(!metrics.is_performance_good());
    }

    #[test]
    fn test_frame_stats() {
        let mut state = RenderState::new();

        // フレーム開始前は統計なし
        assert!(state.get_frame_stats().is_none());

        // フレーム開始
        state.begin_frame();
        std::thread::sleep(Duration::from_millis(1));

        // 統計が取得できる
        let stats = state.get_frame_stats().unwrap();
        assert_eq!(stats.frame_number, 1);
        assert!(stats.frame_time > Duration::ZERO);

        state.end_frame();
    }
}
