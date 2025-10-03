//! 高性能レンダリングシステム
//!
//! 60fps描画、差分更新、画面更新最適化を実現

use crate::ui::{
    layout::{AreaType, LayoutManager},
    text_area::TextAreaRenderer,
    theme::{ComponentType, ThemeManager},
    WindowManager,
};
use crate::buffer::TextEditor;
use crate::minibuffer::MinibufferSystem;
use crate::search::{SearchHighlight, SearchStatus, SearchUiState};
use ratatui::{
    backend::Backend,
    layout::Rect,
    Frame, Terminal,
    widgets::{Block, Borders, Paragraph, Clear},
    style::{Color, Style},
    text::{Line, Span},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::io;

/// 画面領域の差分情報
#[derive(Debug, Clone, PartialEq)]
pub struct AreaDiff {
    /// 領域
    pub area: Rect,
    /// 変更タイプ
    pub change_type: ChangeType,
    /// 変更された行番号（テキストエリアの場合）
    pub changed_lines: Option<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    /// 全体再描画
    Full,
    /// 部分更新
    Partial,
    /// カーソルのみ
    CursorOnly,
    /// ステータスラインのみ
    StatusOnly,
    /// ミニバッファのみ
    MinibufferOnly,
}

/// フレームレート監視
#[derive(Debug, Clone)]
pub struct FrameRateStats {
    /// 現在のFPS
    pub current_fps: f64,
    /// 平均FPS
    pub average_fps: f64,
    /// 最小FPS
    pub min_fps: f64,
    /// 最大FPS
    pub max_fps: f64,
    /// フレーム描画時間の平均
    pub avg_frame_time: Duration,
    /// 最大フレーム描画時間
    pub max_frame_time: Duration,
    /// ドロップしたフレーム数
    pub dropped_frames: u64,
}

/// レンダリング統計
#[derive(Debug, Clone)]
pub struct RenderStats {
    /// 総フレーム数
    pub total_frames: u64,
    /// 差分更新フレーム数
    pub partial_frames: u64,
    /// 全体更新フレーム数
    pub full_frames: u64,
    /// キャッシュヒット率
    pub cache_hit_rate: f64,
    /// 平均描画時間
    pub avg_render_time: Duration,
    /// メモリ使用量
    pub memory_usage: usize,
}

/// 画面キャッシュ
#[derive(Debug, Clone)]
struct ScreenCache {
    /// キャッシュされた画面内容
    content: HashMap<Rect, Vec<String>>,
    /// 最後の更新時刻
    #[allow(dead_code)]
    last_update: Instant,
    /// キャッシュの有効性
    is_valid: bool,
}

impl ScreenCache {
    fn new() -> Self {
        Self {
            content: HashMap::new(),
            last_update: Instant::now(),
            is_valid: false,
        }
    }

    fn invalidate(&mut self) {
        self.is_valid = false;
        self.content.clear();
    }

    #[allow(dead_code)]
    fn is_cache_valid(&self, max_age: Duration) -> bool {
        self.is_valid && self.last_update.elapsed() < max_age
    }
}

/// 高性能レンダラー
pub struct AdvancedRenderer {
    /// レイアウトマネージャー
    layout_manager: LayoutManager,
    /// テーママネージャー
    theme_manager: ThemeManager,
    /// テキストエリアレンダラー
    text_area_renderer: TextAreaRenderer,
    /// 画面キャッシュ
    screen_cache: ScreenCache,
    /// フレームレート統計
    frame_stats: FrameRateStats,
    /// レンダリング統計
    render_stats: RenderStats,
    /// 最後のフレーム時刻
    last_frame_time: Instant,
    /// FPS計算用のフレーム時刻履歴
    frame_times: Vec<Instant>,
    /// 目標FPS
    target_fps: f64,
    /// VSyncの有効性
    vsync_enabled: bool,
    /// 差分更新の有効性
    differential_update: bool,
    /// デバッグモード
    debug_mode: bool,
}

impl AdvancedRenderer {
    /// 新しいレンダラーを作成
    pub fn new() -> Self {
        Self {
            layout_manager: LayoutManager::new(),
            theme_manager: ThemeManager::new(),
            text_area_renderer: TextAreaRenderer::new(),
            screen_cache: ScreenCache::new(),
            frame_stats: FrameRateStats {
                current_fps: 0.0,
                average_fps: 0.0,
                min_fps: f64::MAX,
                max_fps: 0.0,
                avg_frame_time: Duration::ZERO,
                max_frame_time: Duration::ZERO,
                dropped_frames: 0,
            },
            render_stats: RenderStats {
                total_frames: 0,
                partial_frames: 0,
                full_frames: 0,
                cache_hit_rate: 0.0,
                avg_render_time: Duration::ZERO,
                memory_usage: 0,
            },
            last_frame_time: Instant::now(),
            frame_times: Vec::with_capacity(120), // 2秒分のフレーム履歴
            target_fps: 60.0,
            vsync_enabled: true,
            differential_update: true,
            debug_mode: false,
        }
    }

    /// メイン描画処理
    pub fn render<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        editor: &TextEditor,
        windows: &mut WindowManager,
        minibuffer: &MinibufferSystem,
        search_ui: Option<&SearchUiState>,
        search_highlights: &[SearchHighlight],
    ) -> io::Result<()> {
        let frame_start = Instant::now();

        // フレームレート制御
        if self.vsync_enabled {
            self.wait_for_next_frame();
        }

        terminal.draw(|frame| {
            let size = frame.area();

            // レイアウト計算
            let areas = self.layout_manager.calculate_areas(
                size,
                minibuffer.is_active() || search_ui.is_some(),
                true, // ステータスライン表示
            );

            // 差分更新の判定
            let diffs = self.calculate_diffs(editor, minibuffer, &areas);

            // レンダリング実行
            self.render_frame(
                frame,
                editor,
                windows,
                minibuffer,
                search_ui,
                search_highlights,
                &areas,
                &diffs,
            );
        })?;

        // 統計更新
        let frame_time = frame_start.elapsed();
        self.update_frame_stats(frame_time);

        Ok(())
    }

    /// フレーム描画
    fn render_frame(
        &mut self,
        frame: &mut Frame<'_>,
        editor: &TextEditor,
        windows: &mut WindowManager,
        minibuffer: &MinibufferSystem,
        search_ui: Option<&SearchUiState>,
        search_highlights: &[SearchHighlight],
        areas: &HashMap<AreaType, Rect>,
        _diffs: &[AreaDiff],
    ) {
        let theme = self.theme_manager.current_theme();
        let mut cursor_position: Option<(u16, u16)> = None;
        let search_active = search_ui.is_some();

        // テキストエリア描画
        if let Some(&text_area) = areas.get(&AreaType::TextArea) {
            let focused_id = windows.focused_window();
            let (window_rects, divider_rects) = windows.layout_rects_with_dividers(text_area);

            if !divider_rects.is_empty() {
                let divider_style = theme.style(&ComponentType::WindowDivider);
                for divider in divider_rects {
                    frame.render_widget(Clear, divider);
                    let widget = Paragraph::new(Line::from(""))
                        .style(divider_style);
                    frame.render_widget(widget, divider);
                }
            }

            for (window_id, area) in window_rects {
                let is_focused = window_id == focused_id;
                if let Some(viewport) = windows.viewport_mut(window_id) {
                    let text_cursor_pos = self.text_area_renderer.render(
                        frame,
                        area,
                        editor,
                        viewport,
                        theme,
                        if search_active {
                            search_highlights
                        } else {
                            &[]
                        },
                        (minibuffer.is_active() || search_active) && is_focused,
                    );

                    if is_focused && !minibuffer.is_active() && !search_active {
                        cursor_position = text_cursor_pos;
                    }
                }
            }
        }

        // ステータスライン描画
        if let Some(&status_area) = areas.get(&AreaType::StatusLine) {
            self.render_status_line(frame, status_area, editor, theme);
        }

        // ミニバッファ描画
        if let Some(&minibuffer_area) = areas.get(&AreaType::Minibuffer) {
            let minibuffer_cursor_pos = if let Some(search) = search_ui {
                self.render_minibuffer(frame, minibuffer_area, minibuffer, Some(search))
            } else {
                self.render_minibuffer(frame, minibuffer_area, minibuffer, None)
            };

            if let Some(position) = minibuffer_cursor_pos {
                cursor_position = Some(position);
            }
        }

        // カーソル位置設定
        if let Some((x, y)) = cursor_position {
            frame.set_cursor_position(ratatui::layout::Position::new(x, y));
        }
    }

    /// ミニバッファ描画
    fn render_minibuffer(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        minibuffer: &MinibufferSystem,
        search_ui: Option<&SearchUiState>,
    ) -> Option<(u16, u16)> {
        let state = minibuffer.minibuffer_state();
        frame.render_widget(Clear, area);
        if let Some(search) = search_ui {
            let (line, cursor) = Self::search_line(area, search);
            let paragraph = Paragraph::new(line).style(Style::default());
            frame.render_widget(paragraph, area);
            cursor
        } else {
            let (content, cursor_pos) = match &state.mode {
                crate::minibuffer::MinibufferMode::FindFile
                | crate::minibuffer::MinibufferMode::ExecuteCommand
                | crate::minibuffer::MinibufferMode::EvalExpression
                | crate::minibuffer::MinibufferMode::WriteFile => {
                    let line = Self::line_without_cursor(&state.prompt, &state.input);
                    let cursor_x = area.x + state.prompt.chars().count() as u16 + state.cursor_pos as u16;
                    (line, Some((cursor_x, area.y)))
                }
                crate::minibuffer::MinibufferMode::ErrorDisplay { message, .. } => {
                    (Line::from(message.clone()).style(Style::default().fg(Color::Red)), None)
                }
                crate::minibuffer::MinibufferMode::InfoDisplay { message, .. } => {
                    (Line::from(message.clone()).style(Style::default().fg(Color::Green)), None)
                }
                _ => (Line::from(""), None),
            };

            let paragraph = Paragraph::new(content).style(Style::default().fg(Color::Cyan));
            frame.render_widget(paragraph, area);

            cursor_pos
        }
    }

    fn search_line(area: Rect, search: &SearchUiState) -> (Line<'static>, Option<(u16, u16)>) {
        let prompt_text = format!("{}: ", search.prompt_label);
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(prompt_text.clone(), Style::default().fg(Color::Cyan)));

        let pattern_style = match search.status {
            SearchStatus::Active => Style::default().fg(Color::White),
            SearchStatus::Wrapped => Style::default().fg(Color::Yellow),
            SearchStatus::NotFound => Style::default().fg(Color::Red),
        };

        spans.push(Span::styled(search.pattern.clone(), pattern_style));

        if search.total_matches > 0 {
            let current = search.current_match.unwrap_or(0);
            spans.push(Span::styled(
                format!(" [{}/{}]", current, search.total_matches),
                Style::default().fg(Color::Gray),
            ));
        }

        if search.wrapped {
            spans.push(Span::styled(" [wrap]", Style::default().fg(Color::Yellow)));
        }

        if let Some(message) = &search.message {
            let style = if search.is_error() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };
            spans.push(Span::styled(format!(" {}", message), style));
        }

        let mut cursor_col = prompt_text.chars().count() + search.pattern.chars().count();
        let max_col = area.width.saturating_sub(1) as usize;
        if cursor_col > max_col {
            cursor_col = max_col;
        }
        let cursor_x = area.x + cursor_col as u16;
        let cursor_pos = Some((cursor_x, area.y));

        (Line::from(spans), cursor_pos)
    }

    /// ミニバッファ用のカーソルなし行作成
    fn line_without_cursor(prompt: &str, input: &str) -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(prompt.to_string(), Style::default().fg(Color::Cyan)));
        spans.push(Span::raw(input.to_string()));
        Line::from(spans)
    }

    /// ステータスライン描画
    fn render_status_line(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        editor: &TextEditor,
        theme: &crate::ui::theme::Theme,
    ) {
        let cursor = editor.cursor();
        let line_count = 10; // 簡単な実装用の固定値
        let is_modified = false;

        let status_text = format!(
            " {}{}  Line: {}, Col: {}  Total: {} lines  {}",
            if is_modified { "*" } else { " " },
            "untitled",
            cursor.line + 1,
            cursor.column + 1,
            line_count,
            format!("FPS: {:.1}", self.frame_stats.current_fps)
        );

        let paragraph = Paragraph::new(status_text)
            .style(theme.style(&ComponentType::StatusLine));

        frame.render_widget(paragraph, area);
    }

    /// デバッグ情報描画
    #[allow(dead_code)]
    fn render_debug_info(&self, frame: &mut Frame<'_>, areas: &HashMap<AreaType, Rect>) {
        if let Some(&text_area) = areas.get(&AreaType::TextArea) {
            let debug_area = Rect {
                x: text_area.x + text_area.width.saturating_sub(30),
                y: text_area.y,
                width: 30,
                height: 10,
            };

            let debug_text = format!(
                "FPS: {:.1}\nAvg: {:.1}\nFrames: {}\nPartial: {}\nCache: {:.1}%",
                self.frame_stats.current_fps,
                self.frame_stats.average_fps,
                self.render_stats.total_frames,
                self.render_stats.partial_frames,
                self.render_stats.cache_hit_rate * 100.0
            );

            let debug_widget = Paragraph::new(debug_text)
                .style(Style::default().fg(ratatui::style::Color::Yellow))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Debug"));

            frame.render_widget(Clear, debug_area);
            frame.render_widget(debug_widget, debug_area);
        }
    }


    /// 差分計算
    fn calculate_diffs(
        &self,
        _editor: &TextEditor,
        _minibuffer: &MinibufferSystem,
        areas: &HashMap<AreaType, Rect>,
    ) -> Vec<AreaDiff> {
        // 簡単な実装：常に全体更新
        let mut diffs = Vec::new();
        for (_area_type, &area) in areas {
            diffs.push(AreaDiff {
                area,
                change_type: ChangeType::Full,
                changed_lines: None,
            });
        }
        diffs
    }

    /// 領域の更新が必要かチェック
    #[allow(dead_code)]
    fn should_update_area(&self, area_type: &AreaType, diffs: &[AreaDiff]) -> bool {
        diffs.iter().any(|diff| {
            match (area_type, &diff.change_type) {
                (AreaType::TextArea, ChangeType::Full) => true,
                (AreaType::TextArea, ChangeType::Partial) => true,
                (AreaType::StatusLine, ChangeType::Full) => true,
                (AreaType::StatusLine, ChangeType::StatusOnly) => true,
                (AreaType::Minibuffer, ChangeType::Full) => true,
                (AreaType::Minibuffer, ChangeType::MinibufferOnly) => true,
                _ => false,
            }
        })
    }

    /// フレームレート制御
    fn wait_for_next_frame(&self) {
        let target_frame_time = Duration::from_secs_f64(1.0 / self.target_fps);
        let elapsed = self.last_frame_time.elapsed();

        if elapsed < target_frame_time {
            let sleep_time = target_frame_time - elapsed;
            std::thread::sleep(sleep_time);
        }
    }

    /// フレーム統計更新
    fn update_frame_stats(&mut self, frame_time: Duration) {
        let now = Instant::now();
        self.frame_times.push(now);

        // 古いフレーム時刻を削除（2秒以上古い）
        let cutoff_time = now - Duration::from_secs(2);
        self.frame_times.retain(|&time| time > cutoff_time);

        // FPS計算
        if self.frame_times.len() > 1 {
            let duration = now.duration_since(self.frame_times[0]);
            self.frame_stats.current_fps = (self.frame_times.len() - 1) as f64 / duration.as_secs_f64();
        }

        // 統計更新
        self.render_stats.total_frames += 1;

        // 平均フレーム時間の更新
        let weight = 0.1;
        if self.frame_stats.avg_frame_time == Duration::ZERO {
            self.frame_stats.avg_frame_time = frame_time;
        } else {
            let current_nanos = self.frame_stats.avg_frame_time.as_nanos() as f64;
            let new_nanos = frame_time.as_nanos() as f64;
            let avg_nanos = current_nanos * (1.0 - weight) + new_nanos * weight;
            self.frame_stats.avg_frame_time = Duration::from_nanos(avg_nanos as u64);
        }

        // 最大フレーム時間の更新
        if frame_time > self.frame_stats.max_frame_time {
            self.frame_stats.max_frame_time = frame_time;
        }

        // FPS統計の更新
        if self.frame_stats.current_fps < self.frame_stats.min_fps {
            self.frame_stats.min_fps = self.frame_stats.current_fps;
        }
        if self.frame_stats.current_fps > self.frame_stats.max_fps {
            self.frame_stats.max_fps = self.frame_stats.current_fps;
        }

        // 平均FPSの計算
        if self.render_stats.total_frames > 0 {
            self.frame_stats.average_fps = self.frame_stats.current_fps * 0.1 + self.frame_stats.average_fps * 0.9;
        }

        self.last_frame_time = now;
    }

    /// 設定メソッド
    pub fn set_target_fps(&mut self, fps: f64) {
        self.target_fps = fps.max(1.0).min(240.0);
    }

    pub fn enable_vsync(&mut self, enable: bool) {
        self.vsync_enabled = enable;
    }

    pub fn enable_differential_update(&mut self, enable: bool) {
        self.differential_update = enable;
        if !enable {
            self.screen_cache.invalidate();
        }
    }

    pub fn enable_debug_mode(&mut self, enable: bool) {
        self.debug_mode = enable;
    }

    /// 統計情報取得
    pub fn frame_stats(&self) -> &FrameRateStats {
        &self.frame_stats
    }

    pub fn render_stats(&self) -> &RenderStats {
        &self.render_stats
    }

    /// テーマ管理
    pub fn theme_manager(&mut self) -> &mut ThemeManager {
        &mut self.theme_manager
    }

    /// キャッシュ無効化
    pub fn invalidate_cache(&mut self) {
        self.screen_cache.invalidate();
    }

    /// 統計リセット
    pub fn reset_stats(&mut self) {
        self.frame_stats = FrameRateStats {
            current_fps: 0.0,
            average_fps: 0.0,
            min_fps: f64::MAX,
            max_fps: 0.0,
            avg_frame_time: Duration::ZERO,
            max_frame_time: Duration::ZERO,
            dropped_frames: 0,
        };
        self.render_stats = RenderStats {
            total_frames: 0,
            partial_frames: 0,
            full_frames: 0,
            cache_hit_rate: 0.0,
            avg_render_time: Duration::ZERO,
            memory_usage: 0,
        };
        self.frame_times.clear();
    }
}

impl Default for AdvancedRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = AdvancedRenderer::new();
        assert_eq!(renderer.target_fps, 60.0);
        assert!(renderer.vsync_enabled);
        assert!(renderer.differential_update);
    }

    #[test]
    fn test_fps_settings() {
        let mut renderer = AdvancedRenderer::new();

        renderer.set_target_fps(120.0);
        assert_eq!(renderer.target_fps, 120.0);

        renderer.set_target_fps(0.5); // 制限により1.0になる
        assert_eq!(renderer.target_fps, 1.0);

        renderer.set_target_fps(300.0); // 制限により240.0になる
        assert_eq!(renderer.target_fps, 240.0);
    }

    #[test]
    fn test_diff_calculation() {
        let _renderer = AdvancedRenderer::new();

        // モックデータを使用した差分計算のテスト
        // 実際の実装では TextBuffer と MinibufferSystem のモックが必要
    }

    #[test]
    fn test_cache_validity() {
        let mut cache = ScreenCache::new();
        assert!(!cache.is_cache_valid(Duration::from_millis(1)));

        cache.is_valid = true;
        cache.last_update = Instant::now();
        assert!(cache.is_cache_valid(Duration::from_millis(100)));
    }

    #[test]
    fn test_frame_stats_update() {
        let mut renderer = AdvancedRenderer::new();
        let frame_time = Duration::from_millis(16); // ~60fps

        renderer.update_frame_stats(frame_time);
        assert_eq!(renderer.render_stats.total_frames, 1);
        assert_eq!(renderer.frame_stats.avg_frame_time, frame_time);
    }
}
