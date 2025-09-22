# TUIレイアウト設計仕様

## 概要

Altre エディタMVPにおけるratatuiベースのTUIレイアウト設計。効率的な画面構成とレスポンシブな描画パフォーマンスを実現する。

## 設計方針

### 基本原則
1. **シンプルさ優先**: MVPに必要な要素のみを配置
2. **拡張性確保**: 将来のウィンドウ分割機能に対応
3. **パフォーマンス重視**: 差分更新による効率的な描画
4. **アクセシビリティ**: 色覚対応とキーボード操作重視

### QA回答に基づく要件
- **最小サイズ**: 60x15（基本的な編集作業が可能）
- **色対応**: 16色（一般的な色分けが可能）
- **日本語対応**: 基本レベル（一般的な全角文字を正確に計算）

## 画面レイアウト構成

### 全体構成
```
┌────────────────────────────────────────────────────────────┐ ← Terminal Window
│ ┌────────────────────────────────────────────────────────┐ │
│ │                  Minibuffer Area                       │ │ ← 3 lines (QA: 画面上部)
│ └────────────────────────────────────────────────────────┘ │
│ ┌────────────────────────────────────────────────────────┐ │
│ │                                                        │ │
│ │                                                        │ │
│ │                Main Editor Area                        │ │ ← Remaining space
│ │                                                        │ │
│ │                                                        │ │
│ └────────────────────────────────────────────────────────┘ │
│ ┌────────────────────────────────────────────────────────┐ │
│ │                   Mode Line                            │ │ ← 1 line
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

### 詳細レイアウト仕様

#### 1. ミニバッファエリア（上部、3行）
- **位置**: 画面最上部（QA Q7の回答）
- **高さ**: 3行固定
  - 1行目: プロンプト + 入力テキスト
  - 2-3行目: 補完候補表示（最大50候補、QA Q8の回答）
- **表示モード**:
  - `Inactive`: 非表示
  - `FindFile`: "Find file: " + 入力 + 補完
  - `ExecuteCommand`: "M-x " + 入力 + 補完
  - `ErrorDisplay`: エラーメッセージ（5秒間、QA Q10の回答）
  - `InfoDisplay`: 情報メッセージ（3秒間）

#### 2. メインエディタエリア（中央、可変）
- **位置**: ミニバッファとモードラインの間
- **内容**:
  - テキストバッファ内容
  - カーソル表示
  - 行番号（左端）
  - スクロールインジケータ（右端）
- **スクロール**: 垂直・水平スクロール対応

#### 3. モードライン（下部、1行）
- **位置**: 画面最下部
- **内容**:
  - ファイル名
  - 保存状態（変更有無）
  - カーソル位置（行:列）
  - エンコーディング情報

### 最小サイズ対応（60x15）

```
60文字幅 x 15行の場合:
┌──────────────────────────────────────────────────────────┐
│ Find file: /path/to/file.txt                             │ ← Minibuffer (1行)
│ [1] /path/to/file1.txt  [2] /path/to/file2.txt         │ ← Completion (1行)
│ [3] /path/to/file3.txt                                  │ ← Completion (1行)
├──────────────────────────────────────────────────────────┤
│  1 │ テキストの内容がここに表示される。日本語の表示も  │ ← Editor (10行)
│  2 │ 正確に計算される。                                   │
│  3 │ カーソル位置: █                                      │
│  4 │                                                      │
│  5 │                                                      │
│  6 │                                                      │
│  7 │                                                      │
│  8 │                                                      │
│  9 │                                                      │
│ 10 │                                                      │
│ 11 │                                                      │
│ 12 │                                                      │
├──────────────────────────────────────────────────────────┤
│ test.txt [Modified] 3:15 UTF-8                          │ ← Mode line (1行)
└──────────────────────────────────────────────────────────┘
```

## 描画コンポーネント設計

### 1. MinibufferWidget
```rust
pub struct MinibufferWidget<'a> {
    state: &'a MinibufferState,
    style: &'a MinibufferStyle,
}

impl<'a> Widget for MinibufferWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match &self.state.mode {
            MinibufferMode::Inactive => {
                // 空白表示
            }
            MinibufferMode::FindFile | MinibufferMode::ExecuteCommand => {
                self.render_input_mode(area, buf);
            }
            MinibufferMode::ErrorDisplay { message, .. } => {
                self.render_message(area, buf, message, Color::Red);
            }
            MinibufferMode::InfoDisplay { message, .. } => {
                self.render_message(area, buf, message, Color::Green);
            }
        }
    }
}
```

### 2. EditorWidget
```rust
pub struct EditorWidget<'a> {
    buffer: &'a TextBuffer,
    cursor: CursorPosition,
    viewport: ViewportState,
    style: &'a EditorStyle,
}

impl<'a> Widget for EditorWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 行番号表示
        self.render_line_numbers(area, buf);

        // テキスト内容表示
        self.render_text_content(area, buf);

        // カーソル表示
        self.render_cursor(area, buf);

        // スクロールバー表示
        self.render_scrollbar(area, buf);
    }
}
```

### 3. ModeLineWidget
```rust
pub struct ModeLineWidget<'a> {
    file_info: &'a FileInfo,
    cursor_info: &'a CursorInfo,
    style: &'a ModeLineStyle,
}

impl<'a> Widget for ModeLineWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = format!(
            "{} [{}] {}:{} {}",
            self.file_info.name,
            if self.file_info.modified { "Modified" } else { "Saved" },
            self.cursor_info.line,
            self.cursor_info.column,
            self.file_info.encoding
        );

        Paragraph::new(content)
            .style(self.style.base_style)
            .render(area, buf);
    }
}
```

## 色彩設計（16色対応）

### 基本色パレット
```rust
pub struct ColorScheme {
    // 基本色（QA Q14: 16色対応）
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

impl Default for ColorScheme {
    fn default() -> Self {
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
            minibuffer_selected: Color::Black, // 背景: Blue

            // メッセージ色
            error_message: Color::Red,
            warning_message: Color::Yellow,
            info_message: Color::Green,

            // ボーダー色
            border_normal: Color::DarkGray,
            border_focus: Color::Cyan,
        }
    }
}
```

## 日本語文字幅対応（基本レベル）

### 文字幅計算
```rust
pub fn char_width(ch: char) -> usize {
    match ch {
        // ASCII文字
        '\x00'..='\x7F' => 1,

        // 制御文字
        '\x80'..='\x9F' => 0,

        // 全角文字の基本的な判定（QA Q15の回答）
        // ひらがな・カタカナ
        '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => 2,

        // CJK統合漢字
        '\u{4E00}'..='\u{9FFF}' => 2,

        // 全角記号
        '\u{FF01}'..='\u{FF60}' => 2,

        // その他は東アジア幅特性を簡易判定
        _ => {
            if unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1) == 2 {
                2
            } else {
                1
            }
        }
    }
}

pub fn string_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

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
```

## レスポンシブ対応

### ターミナルサイズ検出と調整
```rust
pub struct LayoutManager {
    min_width: u16,
    min_height: u16,
    current_size: (u16, u16),
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            min_width: 60,   // QA Q13の回答
            min_height: 15,  // QA Q13の回答
            current_size: (0, 0),
        }
    }

    pub fn update_size(&mut self, width: u16, height: u16) -> Result<(), LayoutError> {
        if width < self.min_width || height < self.min_height {
            return Err(LayoutError::ScreenTooSmall { width, height });
        }

        self.current_size = (width, height);
        Ok(())
    }

    pub fn calculate_areas(&self) -> LayoutAreas {
        let (width, height) = self.current_size;

        // ミニバッファエリア（上部3行）
        let minibuffer_height = if height >= 20 { 3 } else { 2 };
        let minibuffer_area = Rect {
            x: 0,
            y: 0,
            width,
            height: minibuffer_height,
        };

        // モードラインエリア（下部1行）
        let modeline_area = Rect {
            x: 0,
            y: height - 1,
            width,
            height: 1,
        };

        // エディタエリア（残り）
        let editor_area = Rect {
            x: 0,
            y: minibuffer_height,
            width,
            height: height - minibuffer_height - 1,
        };

        LayoutAreas {
            minibuffer: minibuffer_area,
            editor: editor_area,
            modeline: modeline_area,
        }
    }
}

pub struct LayoutAreas {
    pub minibuffer: Rect,
    pub editor: Rect,
    pub modeline: Rect,
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("Screen size too small: {width}x{height}, minimum required: 60x15")]
    ScreenTooSmall { width: u16, height: u16 },
}
```

## 描画パフォーマンス要件

### 差分更新戦略
```rust
pub struct RenderState {
    last_frame: Option<Buffer>,
    dirty_regions: Vec<Rect>,
}

impl RenderState {
    pub fn mark_dirty(&mut self, area: Rect) {
        self.dirty_regions.push(area);
    }

    pub fn should_redraw(&self, area: Rect) -> bool {
        self.dirty_regions.iter().any(|dirty| dirty.intersects(area))
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_regions.clear();
    }
}

pub struct PerformanceMetrics {
    pub frame_time: Duration,
    pub render_calls: usize,
    pub buffer_updates: usize,
}

// パフォーマンス目標
const TARGET_FPS: u32 = 60;
const MAX_FRAME_TIME: Duration = Duration::from_millis(16); // 1/60秒
```

### 最適化戦略
1. **部分更新**: 変更されたテキスト行のみ再描画
2. **スクロール最適化**: ビューポート外のテキストは描画しない
3. **文字幅キャッシュ**: 計算済み文字幅をキャッシュ
4. **バッファプーリング**: ratatuiバッファの再利用

## 統合例：メインアプリケーション描画ループ

```rust
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout},
};

pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    layout_manager: LayoutManager,
    render_state: RenderState,
    color_scheme: ColorScheme,
}

impl TuiApp {
    pub fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            layout_manager: LayoutManager::new(),
            render_state: RenderState::new(),
            color_scheme: ColorScheme::default(),
        })
    }

    pub fn draw(&mut self, app_state: &AppState) -> Result<()> {
        let start_time = Instant::now();

        self.terminal.draw(|frame| {
            let size = frame.size();

            // サイズ更新
            if let Err(e) = self.layout_manager.update_size(size.width, size.height) {
                // エラー表示
                self.draw_error(frame, &e);
                return;
            }

            // レイアウト計算
            let areas = self.layout_manager.calculate_areas();

            // 各コンポーネント描画
            self.draw_minibuffer(frame, &areas.minibuffer, &app_state.minibuffer);
            self.draw_editor(frame, &areas.editor, &app_state.editor);
            self.draw_modeline(frame, &areas.modeline, &app_state.file_info);
        })?;

        // パフォーマンス測定
        let frame_time = start_time.elapsed();
        if frame_time > MAX_FRAME_TIME {
            log::warn!("Frame time exceeded target: {:?}", frame_time);
        }

        Ok(())
    }

    fn draw_minibuffer(&self, frame: &mut Frame, area: &Rect, state: &MinibufferState) {
        let widget = MinibufferWidget::new(state, &self.color_scheme);
        frame.render_widget(widget, *area);
    }

    fn draw_editor(&self, frame: &mut Frame, area: &Rect, state: &EditorState) {
        let widget = EditorWidget::new(
            &state.buffer,
            state.cursor,
            state.viewport,
            &self.color_scheme
        );
        frame.render_widget(widget, *area);
    }

    fn draw_modeline(&self, frame: &mut Frame, area: &Rect, file_info: &FileInfo) {
        let widget = ModeLineWidget::new(file_info, &self.color_scheme);
        frame.render_widget(widget, *area);
    }

    fn draw_error(&self, frame: &mut Frame, error: &LayoutError) {
        let error_text = format!("Error: {}", error);
        let paragraph = Paragraph::new(error_text)
            .style(Style::default().fg(Color::Red).bg(Color::Black));
        frame.render_widget(paragraph, frame.size());
    }
}
```

## テスト戦略

### 視覚的回帰テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_size_layout() {
        let mut layout_manager = LayoutManager::new();
        layout_manager.update_size(60, 15).unwrap();

        let areas = layout_manager.calculate_areas();

        assert_eq!(areas.minibuffer.height, 2); // 最小サイズでは2行
        assert_eq!(areas.modeline.height, 1);
        assert_eq!(areas.editor.height, 12); // 15 - 2 - 1
    }

    #[test]
    fn test_japanese_char_width() {
        assert_eq!(char_width('a'), 1);
        assert_eq!(char_width('あ'), 2);
        assert_eq!(char_width('漢'), 2);
        assert_eq!(char_width('🎉'), 2);

        assert_eq!(string_width("hello"), 5);
        assert_eq!(string_width("こんにちは"), 10);
        assert_eq!(string_width("hello世界"), 9);
    }

    #[test]
    fn test_color_scheme_accessibility() {
        let scheme = ColorScheme::default();

        // エラー色は背景と十分なコントラストを持つ
        assert_ne!(scheme.error_message, scheme.background);
        assert_ne!(scheme.info_message, scheme.background);
    }
}
```

## 将来の拡張計画

### フェーズ2: 高度なレイアウト
- 分割ウィンドウサポート
- タブ機能
- サイドバー（ファイル階層表示）
- ポップアップウィンドウ

### フェーズ3: 視覚的強化
- シンタックスハイライト
- テーマサポート
- True Color対応（高品質ターミナル向け）
- アニメーション効果

### フェーズ4: アクセシビリティ
- 高コントラストモード
- 文字サイズ調整
- 色覚対応パレット
- スクリーンリーダー対応

この設計により、MVP段階での実用的なTUIと将来の高度な機能への拡張性を両立させる。