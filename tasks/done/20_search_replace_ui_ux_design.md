# 検索・置換UI/UX設計

## タスク概要
Emacsライクな検索・置換機能のユーザーインターフェースとユーザーエクスペリエンスを設計する。

## 目的
- 直感的で効率的な検索・置換インターフェースの構築
- ミニバッファとの統合による一貫したUX提供
- 視覚的フィードバックによるユーザビリティ向上

## インクリメンタル検索UI設計

### 基本インターフェース構成
```
┌─────────────────────────────────────────────────────────────┐
│ 検索中: pattern_text          [1/5 matches] [wrapped]       │ <- ミニバッファ
├─────────────────────────────────────────────────────────────┤
│ def function_name():                                        │
│     return "Hello, [pattern_text]!"  # <- 現在のマッチ強調  │
│                                                             │
│ def another_function():                                     │
│     print("[pattern_text] world")    # <- その他のマッチ    │
│     value = pattern_text + "_suffix"                        │
│                                                             │
│ # [pattern_text] in comment         # <- その他のマッチ    │
└─────────────────────────────────────────────────────────────┘
```

### 検索状態表示
```rust
/// 検索UI状態
#[derive(Debug, Clone)]
pub struct SearchUIState {
    /// 検索モード（前方・後方・正規表現）
    pub mode: SearchMode,

    /// 検索ステータス
    pub status: SearchStatus,

    /// プロンプトテキスト
    pub prompt: String,

    /// カウンター情報
    pub match_info: MatchInfo,
}

#[derive(Debug, Clone)]
pub enum SearchMode {
    Forward,         // 前方検索（I-search:）
    Backward,        // 後方検索（I-search backward:）
    ForwardRegex,    // 前方正規表現検索（Regexp I-search:）
    BackwardRegex,   // 後方正規表現検索（Regexp I-search backward:）
}

#[derive(Debug, Clone)]
pub enum SearchStatus {
    Searching,       // 通常の検索中
    Failed,          // 検索失敗（赤色表示）
    Wrapped,         // 検索が折り返し（黄色表示）
    Successful,      // 検索成功
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    /// 現在のマッチ番号（1-based）
    pub current: usize,

    /// 総マッチ数
    pub total: usize,

    /// 検索が折り返したか
    pub wrapped: bool,
}
```

### ミニバッファ統合
```rust
/// 検索用ミニバッファレンダラー
pub struct SearchMinibufferRenderer {
    /// カラーテーマ
    theme: SearchColorTheme,
}

#[derive(Debug, Clone)]
pub struct SearchColorTheme {
    /// 通常の検索プロンプト色
    pub normal_prompt: Color,

    /// 検索失敗時のプロンプト色
    pub failed_prompt: Color,

    /// 検索成功時のプロンプト色
    pub success_prompt: Color,

    /// カウンター表示色
    pub counter_color: Color,

    /// 折り返し警告色
    pub wrapped_color: Color,
}

impl SearchMinibufferRenderer {
    /// 検索プロンプトをレンダリング
    pub fn render_search_prompt(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &SearchUIState,
        pattern: &str,
    ) {
        let prompt_text = self.build_prompt_text(state, pattern);
        let prompt_style = self.get_prompt_style(&state.status);

        // プロンプトとパターンを表示
        let paragraph = Paragraph::new(prompt_text)
            .style(prompt_style)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);

        // カーソル位置計算と表示
        if let Some(cursor_pos) = self.calculate_cursor_position(area, &prompt_text) {
            frame.set_cursor(cursor_pos.0, cursor_pos.1);
        }
    }

    /// プロンプトテキスト構築
    fn build_prompt_text(&self, state: &SearchUIState, pattern: &str) -> String {
        let mode_prompt = match state.mode {
            SearchMode::Forward => "検索中",
            SearchMode::Backward => "後方検索中",
            SearchMode::ForwardRegex => "正規表現検索中",
            SearchMode::BackwardRegex => "後方正規表現検索中",
        };

        let counter_info = if state.match_info.total > 0 {
            format!(" [{}/{}]", state.match_info.current, state.match_info.total)
        } else if state.status == SearchStatus::Failed {
            " [見つかりません]".to_string()
        } else {
            String::new()
        };

        let wrapped_info = if state.match_info.wrapped {
            " [折り返し]"
        } else {
            ""
        };

        format!("{}: {}{}{}", mode_prompt, pattern, counter_info, wrapped_info)
    }
}
```

## テキストハイライト設計

### ハイライト表示システム
```rust
/// 検索結果ハイライトマネージャー
pub struct SearchHighlightManager {
    /// 現在のハイライト範囲
    highlights: Vec<HighlightRange>,

    /// ハイライトテーマ
    theme: HighlightTheme,

    /// 表示領域の最適化
    viewport: ViewportInfo,
}

#[derive(Debug, Clone)]
pub struct HighlightTheme {
    /// 現在のマッチのハイライト色
    pub current_match_bg: Color,
    pub current_match_fg: Color,

    /// その他のマッチのハイライト色
    pub other_match_bg: Color,
    pub other_match_fg: Color,

    /// 検索失敗時の色（該当なし）
    pub failed_bg: Color,
    pub failed_fg: Color,
}

impl SearchHighlightManager {
    /// 検索結果に基づくハイライト範囲更新
    pub fn update_highlights(
        &mut self,
        matches: &[SearchMatch],
        current_match_index: Option<usize>,
        viewport: ViewportInfo,
    ) {
        self.viewport = viewport;
        self.highlights.clear();

        // 表示範囲内のマッチのみ処理
        for (index, match_result) in matches.iter().enumerate() {
            if self.is_in_viewport(match_result) {
                let highlight_type = if Some(index) == current_match_index {
                    HighlightType::CurrentMatch
                } else {
                    HighlightType::OtherMatch
                };

                self.highlights.push(HighlightRange {
                    start: match_result.start,
                    end: match_result.end,
                    highlight_type,
                });
            }
        }
    }

    /// ハイライトをテキストレンダリングに適用
    pub fn apply_highlights_to_spans<'a>(
        &self,
        text: &'a str,
        base_style: Style,
    ) -> Vec<Span<'a>> {
        let mut spans = Vec::new();
        let mut current_pos = 0;

        // ハイライト範囲でテキストを分割
        for highlight in &self.highlights {
            // ハイライト前の通常テキスト
            if current_pos < highlight.start {
                spans.push(Span::styled(
                    &text[current_pos..highlight.start],
                    base_style
                ));
            }

            // ハイライトされたテキスト
            let highlight_style = self.get_highlight_style(&highlight.highlight_type);
            spans.push(Span::styled(
                &text[highlight.start..highlight.end],
                highlight_style
            ));

            current_pos = highlight.end;
        }

        // 残りの通常テキスト
        if current_pos < text.len() {
            spans.push(Span::styled(&text[current_pos..], base_style));
        }

        spans
    }
}
```

## 置換UI設計

### クエリ置換インターフェース
```
┌─────────────────────────────────────────────────────────────┐
│ 検索文字列を置換文字列で置き換えますか? (y/n/!/q/^)          │ <- ミニバッファ
├─────────────────────────────────────────────────────────────┤
│ def function_name():                                        │
│     return "Hello, [old_text]!"  # <- 置換対象を強調       │
│                          ^^^^^^^^^^^                        │
│                          └─ "new_text" で置換予定          │
│                                                             │
│ def another_function():                                     │
│     print("old_text world")     # <- 次の置換候補          │
│     value = old_text + "_suffix"                            │
└─────────────────────────────────────────────────────────────┘
```

### 置換確認UI
```rust
/// 置換UI状態
#[derive(Debug, Clone)]
pub struct ReplaceUIState {
    /// 現在の置換モード
    pub mode: ReplaceMode,

    /// 置換進行状況
    pub progress: ReplaceProgress,

    /// プレビュー表示設定
    pub preview: ReplacePreview,
}

#[derive(Debug, Clone)]
pub enum ReplaceMode {
    QueryReplace,      // クエリ置換（M-%）
    RegexReplace,      // 正規表現置換（C-M-%）
    RectangleReplace,  // 矩形置換（将来実装）
}

#[derive(Debug, Clone)]
pub struct ReplaceProgress {
    /// 現在の置換位置
    pub current_index: usize,

    /// 総置換候補数
    pub total_candidates: usize,

    /// 実行済み置換数
    pub completed_replacements: usize,

    /// スキップされた置換数
    pub skipped_replacements: usize,
}

#[derive(Debug, Clone)]
pub struct ReplacePreview {
    /// 置換前テキスト
    pub before_text: String,

    /// 置換後テキスト
    pub after_text: String,

    /// 置換範囲の表示
    pub highlight_range: (usize, usize),
}
```

### 置換キー操作ガイド
```rust
/// 置換操作ヘルプ表示
pub struct ReplaceHelpRenderer;

impl ReplaceHelpRenderer {
    /// 置換操作ヘルプをレンダリング
    pub fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(vec![
                Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" または "),
                Span::styled("SPC", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(": 置換して次へ"),
            ]),
            Line::from(vec![
                Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(" または "),
                Span::styled("DEL", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(": 置換せずに次へ"),
            ]),
            Line::from(vec![
                Span::styled("!", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": 残り全て置換"),
            ]),
            Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                Span::raw(" または "),
                Span::styled("Enter", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                Span::raw(": 置換終了"),
            ]),
            Line::from(vec![
                Span::styled("^", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(": 前の置換に戻る"),
            ]),
            Line::from(vec![
                Span::styled("C-g", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Span::raw(": 置換キャンセル"),
            ]),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(Block::default()
                .title("置換操作")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(help_paragraph, area);
    }
}
```

## レスポンシブデザイン

### 画面サイズ対応
```rust
/// 画面サイズに応じたUI調整
pub struct ResponsiveSearchUI {
    /// 最小表示要件
    min_requirements: MinDisplayRequirements,
}

#[derive(Debug, Clone)]
pub struct MinDisplayRequirements {
    /// 最小幅（文字数）
    pub min_width: u16,

    /// 最小高さ（行数）
    pub min_height: u16,

    /// ミニバッファの最小幅
    pub min_minibuffer_width: u16,
}

impl ResponsiveSearchUI {
    /// 画面サイズに基づくレイアウト調整
    pub fn adjust_layout(&self, terminal_size: Rect) -> SearchLayout {
        if terminal_size.width < self.min_requirements.min_width {
            // 幅が狭い場合の調整
            SearchLayout::Compact {
                show_counter: false,
                truncate_pattern: true,
                help_abbreviated: true,
            }
        } else if terminal_size.height < self.min_requirements.min_height {
            // 高さが低い場合の調整
            SearchLayout::Minimal {
                hide_help: true,
                single_line_preview: true,
            }
        } else {
            // 十分な画面サイズの場合
            SearchLayout::Full {
                show_all_info: true,
                detailed_help: true,
                multi_line_preview: true,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchLayout {
    Compact {
        show_counter: bool,
        truncate_pattern: bool,
        help_abbreviated: bool,
    },
    Minimal {
        hide_help: bool,
        single_line_preview: bool,
    },
    Full {
        show_all_info: bool,
        detailed_help: bool,
        multi_line_preview: bool,
    },
}
```

## アクセシビリティ対応

### 視覚的配慮
```rust
/// アクセシビリティ設定
#[derive(Debug, Clone)]
pub struct AccessibilitySettings {
    /// ハイコントラストモード
    pub high_contrast: bool,

    /// 色覚異常対応
    pub colorblind_friendly: bool,

    /// 点滅エフェクトの無効化
    pub disable_blinking: bool,

    /// 音響フィードバック
    pub audio_feedback: bool,
}

impl AccessibilitySettings {
    /// アクセシビリティに配慮した色テーマ生成
    pub fn create_accessible_theme(&self) -> SearchColorTheme {
        if self.high_contrast {
            SearchColorTheme {
                normal_prompt: Color::White,
                failed_prompt: Color::Red,
                success_prompt: Color::Green,
                counter_color: Color::Cyan,
                wrapped_color: Color::Yellow,
            }
        } else if self.colorblind_friendly {
            // 色覚異常に配慮した色選択
            SearchColorTheme {
                normal_prompt: Color::Blue,
                failed_prompt: Color::Rgb(255, 100, 100),  // 明るい赤
                success_prompt: Color::Rgb(100, 255, 100), // 明るい緑
                counter_color: Color::Cyan,
                wrapped_color: Color::Rgb(255, 200, 0),    // オレンジ
            }
        } else {
            self.default_theme()
        }
    }
}
```

## 依存関係
- ratatui（TUIレンダリング）
- TextEditorとの統合
- ミニバッファシステム
- テーマシステム

## 成果物
- 検索UI コンポーネント
- 置換UI コンポーネント
- ハイライトシステム
- レスポンシブレイアウト

## 完了条件
- [ ] インクリメンタル検索UI実装完了
- [ ] 置換確認UI実装完了
- [ ] ハイライトシステム実装完了
- [ ] レスポンシブデザイン実装完了
- [ ] アクセシビリティ対応実装完了

## 進捗記録
- 作成日：2025-01-28
- 状態：設計フェーズ