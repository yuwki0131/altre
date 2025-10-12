# ナビゲーション設計仕様書

## 概要

本文書は、Altreテキストエディタのナビゲーション機能（カーソル移動）の詳細設計を定義する。Emacs風キーバインドと矢印キーの統合、ギャップバッファでの効率的な位置計算、および高速応答性（< 1ms）を実現する設計を提供する。

## 設計目標

1. **高速応答性**: QA.mdの要件「カーソル移動 < 1ms」の確実な実現
2. **Emacs互換性**: 基本的なEmacs風キーバインドとの完全な互換性
3. **UTF-8完全対応**: 日本語・絵文字を含む全Unicode文字での正確な移動
4. **画面表示統合**: ratatuiとの効率的な連携とスムーズなスクロール
5. **ユーザビリティ**: 直感的で予測可能な移動動作

## QA確認事項の回答に基づく設計方針

### Tab文字の表示幅（Q21回答）
- **デフォルト**: 4スペース幅
- **MVP実装**: 固定値として実装
- **将来拡張**: 設定可能な機能として拡張予定

### 長い行でのパフォーマンス（Q22回答）
- **基本方針**: 段階的制限、性能劣化許容
- **短い行（< 1000文字）**: < 1ms目標維持
- **長い行（1000-10000文字）**: < 5ms許容
- **超長い行（> 10000文字）**: < 10ms許容

### 画面外移動時のスクロール（Q23回答）
- **スクロール方針**: 中央配置
- **動作**: カーソルが画面外に移動した際、カーソルを画面中央に配置

## アーキテクチャ設計

### 基本構造

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   KeyBinding    │───▶│   Navigation    │───▶│   GapBuffer     │
│   (入力処理)     │    │   (移動処理)     │    │  (位置管理)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │ CursorPosition  │───▶│ ScreenDisplay   │
                       │  (位置状態)      │    │  (画面更新)      │
                       └─────────────────┘    └─────────────────┘
```

### コンポーネント関係

```rust
/// ナビゲーションシステムの統合
pub struct NavigationSystem {
    /// カーソル位置管理
    cursor: CursorPosition,
    /// 位置計算エンジン
    position_engine: PositionCalculator,
    /// 画面表示管理
    viewport_manager: ViewportManager,
    /// パフォーマンス監視
    performance_monitor: PerformanceMonitor,
}
```

## 1. 基本移動操作設計

### 1.1 キーバインド仕様

```rust
/// ナビゲーション用キーバインド
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationAction {
    /// 基本文字移動
    MoveCharForward,     // C-f, →
    MoveCharBackward,    // C-b, ←

    /// 基本行移動
    MoveLineUp,          // C-p, ↑
    MoveLineDown,        // C-n, ↓

    /// 行内移動
    MoveLineStart,       // C-a
    MoveLineEnd,         // C-e

    /// バッファ全体移動
    MoveBufferStart,     // M-<
    MoveBufferEnd,       // M->

    /// 将来拡張（MVPでは未実装）
    MoveWordForward,     // M-f
    MoveWordBackward,    // M-b
    MoveParagraphUp,     // C-up
    MoveParagraphDown,   // C-down
}
```

### 1.2 キーマッピング統合

```rust
/// 既存のキーバインドシステムとの統合
impl ModernKeyMap {
    fn register_navigation_bindings(
        single: &mut HashMap<Key, Action>,
        cx_prefix: &mut HashMap<Key, Action>
    ) {
        // Emacs風移動
        single.insert(Key::ctrl_f(), Action::Navigate(NavigationAction::MoveCharForward));
        single.insert(Key::ctrl_b(), Action::Navigate(NavigationAction::MoveCharBackward));
        single.insert(Key::ctrl_n(), Action::Navigate(NavigationAction::MoveLineDown));
        single.insert(Key::ctrl_p(), Action::Navigate(NavigationAction::MoveLineUp));
        single.insert(Key::ctrl_a(), Action::Navigate(NavigationAction::MoveLineStart));
        single.insert(Key::ctrl_e(), Action::Navigate(NavigationAction::MoveLineEnd));

        // 矢印キー
        single.insert(Key::arrow_up(), Action::Navigate(NavigationAction::MoveLineUp));
        single.insert(Key::arrow_down(), Action::Navigate(NavigationAction::MoveLineDown));
        single.insert(Key::arrow_left(), Action::Navigate(NavigationAction::MoveCharBackward));
        single.insert(Key::arrow_right(), Action::Navigate(NavigationAction::MoveCharForward));
    }
}
```

## 2. 位置計算システム設計

### 2.1 座標系定義

```rust
/// 位置計算で使用する座標系
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// 文字位置（UTF-8文字単位、0ベース）
    pub char_pos: usize,
    /// 行番号（0ベース）
    pub line: usize,
    /// 列番号（表示列、0ベース）
    pub visual_column: usize,
    /// 論理列番号（文字数、0ベース）
    pub logical_column: usize,
}

/// 表示幅計算（Tab考慮）
impl Position {
    /// 論理列から表示列を計算
    pub fn logical_to_visual_column(logical_col: usize, line_text: &str, tab_width: usize) -> usize {
        let mut visual_col = 0;

        for (i, ch) in line_text.chars().enumerate() {
            if i >= logical_col {
                break;
            }

            if ch == '\t' {
                visual_col += tab_width - (visual_col % tab_width);
            } else {
                visual_col += Self::char_display_width(ch);
            }
        }

        visual_col
    }

    /// 文字の表示幅を計算（QA Q15: 基本対応）
    fn char_display_width(ch: char) -> usize {
        match ch {
            // ASCII文字
            '\u{0000}'..='\u{007F}' => 1,
            // 全角文字（基本的な判定）
            '\u{1100}'..='\u{115F}' |  // ハングル字母
            '\u{2E80}'..='\u{2EFF}' |  // CJK部首補助
            '\u{2F00}'..='\u{2FDF}' |  // 康熙部首
            '\u{3000}'..='\u{303F}' |  // CJK記号
            '\u{3040}'..='\u{309F}' |  // ひらがな
            '\u{30A0}'..='\u{30FF}' |  // カタカナ
            '\u{3100}'..='\u{312F}' |  // 注音字母
            '\u{3130}'..='\u{318F}' |  // ハングル互換字母
            '\u{3190}'..='\u{319F}' |  // 漢文用記号
            '\u{31A0}'..='\u{31BF}' |  // 注音拡張
            '\u{31C0}'..='\u{31EF}' |  // CJKストローク
            '\u{31F0}'..='\u{31FF}' |  // カタカナ拡張
            '\u{3200}'..='\u{32FF}' |  // CJK互換
            '\u{3300}'..='\u{33FF}' |  // CJK互換
            '\u{3400}'..='\u{4DBF}' |  // CJK拡張A
            '\u{4E00}'..='\u{9FFF}' |  // CJK統合漢字
            '\u{A000}'..='\u{A48F}' |  // イ語
            '\u{A490}'..='\u{A4CF}' |  // イ語部首
            '\u{AC00}'..='\u{D7AF}' |  // ハングル音節
            '\u{F900}'..='\u{FAFF}' |  // CJK互換漢字
            '\u{FE10}'..='\u{FE1F}' |  // 縦書き用記号
            '\u{FE30}'..='\u{FE4F}' |  // CJK互換形
            '\u{FE50}'..='\u{FE6F}' |  // 小字形
            '\u{FF00}'..='\u{FFEF}' => 2, // 全角英数・記号
            // 絵文字（基本）
            '\u{1F300}'..='\u{1F5FF}' |
            '\u{1F600}'..='\u{1F64F}' |
            '\u{1F680}'..='\u{1F6FF}' |
            '\u{1F700}'..='\u{1F77F}' |
            '\u{1F780}'..='\u{1F7FF}' |
            '\u{1F800}'..='\u{1F8FF}' |
            '\u{1F900}'..='\u{1F9FF}' |
            '\u{1FA00}'..='\u{1FA6F}' |
            '\u{1FA70}'..='\u{1FAFF}' => 2,
            // その他は1として扱う
            _ => 1,
        }
    }
}
```

### 2.2 効率的な位置計算

```rust
/// 高性能位置計算エンジン
pub struct PositionCalculator {
    /// 行インデックスキャッシュ
    line_index_cache: Vec<usize>,
    /// キャッシュの有効性
    cache_valid: bool,
    /// Tab幅設定
    tab_width: usize,
}

impl PositionCalculator {
    pub fn new() -> Self {
        Self {
            line_index_cache: Vec::new(),
            cache_valid: false,
            tab_width: 4, // QA Q21回答
        }
    }

    /// 高速な文字位置から行・列位置への変換
    pub fn char_pos_to_line_col(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        if !self.cache_valid {
            self.rebuild_line_cache(text);
        }

        // バイナリサーチで行を特定
        let line = match self.line_index_cache.binary_search(&char_pos) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        };

        if line >= self.line_index_cache.len() {
            return Err(NavigationError::InvalidPosition(char_pos));
        }

        let line_start = self.line_index_cache[line];
        let logical_column = char_pos - line_start;

        // 行のテキストを取得して表示列を計算
        let line_text = self.get_line_text(text, line);
        let visual_column = Position::logical_to_visual_column(logical_column, &line_text, self.tab_width);

        Ok(Position {
            char_pos,
            line,
            visual_column,
            logical_column,
        })
    }

    /// 行・列位置から文字位置への変換
    pub fn line_col_to_char_pos(&mut self, text: &str, line: usize, logical_column: usize) -> Result<usize, NavigationError> {
        if !self.cache_valid {
            self.rebuild_line_cache(text);
        }

        if line >= self.line_index_cache.len() {
            return Err(NavigationError::InvalidLine(line));
        }

        let line_start = self.line_index_cache[line];
        let line_text = self.get_line_text(text, line);
        let line_length = line_text.chars().count();

        let clamped_column = logical_column.min(line_length);
        Ok(line_start + clamped_column)
    }

    /// 行インデックスキャッシュの再構築
    fn rebuild_line_cache(&mut self, text: &str) {
        self.line_index_cache.clear();
        self.line_index_cache.push(0); // 最初の行は0から開始

        let mut char_pos = 0;
        for ch in text.chars() {
            char_pos += 1;
            if ch == '\n' {
                self.line_index_cache.push(char_pos);
            }
        }

        self.cache_valid = true;
    }

    /// 指定行のテキストを取得
    fn get_line_text(&self, text: &str, line: usize) -> String {
        text.lines().nth(line).unwrap_or("").to_string()
    }

    /// キャッシュを無効化
    pub fn invalidate_cache(&mut self) {
        self.cache_valid = false;
    }
}
```

## 3. 行移動の詳細設計

### 3.1 上下移動アルゴリズム

```rust
/// 行移動の詳細実装
impl NavigationSystem {
    /// 上移動（C-p, ↑）
    pub fn move_line_up(&mut self, text: &str) -> Result<bool, NavigationError> {
        let current_pos = self.position_engine.char_pos_to_line_col(text, self.cursor.char_pos)?;

        if current_pos.line == 0 {
            return Ok(false); // ファイル先頭で停止
        }

        let target_line = current_pos.line - 1;

        // 列位置保持の実装
        let target_char_pos = self.calculate_target_position_for_line_move(
            text,
            target_line,
            current_pos.logical_column
        )?;

        self.update_cursor_position(target_char_pos, text)?;
        Ok(true)
    }

    /// 下移動（C-n, ↓）
    pub fn move_line_down(&mut self, text: &str) -> Result<bool, NavigationError> {
        let current_pos = self.position_engine.char_pos_to_line_col(text, self.cursor.char_pos)?;
        let total_lines = self.count_total_lines(text);

        if current_pos.line >= total_lines.saturating_sub(1) {
            return Ok(false); // ファイル末尾で停止
        }

        let target_line = current_pos.line + 1;

        let target_char_pos = self.calculate_target_position_for_line_move(
            text,
            target_line,
            current_pos.logical_column
        )?;

        self.update_cursor_position(target_char_pos, text)?;
        Ok(true)
    }

    /// 行移動時の目標位置計算
    fn calculate_target_position_for_line_move(
        &mut self,
        text: &str,
        target_line: usize,
        preferred_column: usize
    ) -> Result<usize, NavigationError> {
        let target_line_text = self.position_engine.get_line_text(text, target_line);
        let target_line_length = target_line_text.chars().count();

        // 短い行での列位置調整
        let actual_column = preferred_column.min(target_line_length);

        self.position_engine.line_col_to_char_pos(text, target_line, actual_column)
    }
}
```

### 3.2 列位置保持メカニズム

```rust
/// 列位置保持のための拡張カーソル情報
#[derive(Debug, Clone)]
pub struct ExtendedCursor {
    /// 基本カーソル情報
    pub position: CursorPosition,
    /// 上下移動時の希望列位置
    pub preferred_column: Option<usize>,
    /// 最後の移動操作
    pub last_movement: Option<NavigationAction>,
}

impl ExtendedCursor {
    /// 上下移動時の列位置保持
    pub fn update_with_line_movement(&mut self, new_position: Position, action: NavigationAction) {
        // 上下移動の場合、希望列位置を保持
        if matches!(action, NavigationAction::MoveLineUp | NavigationAction::MoveLineDown) {
            if self.preferred_column.is_none() {
                self.preferred_column = Some(new_position.logical_column);
            }
        } else {
            // 他の移動操作では希望列位置をリセット
            self.preferred_column = None;
        }

        self.position.char_pos = new_position.char_pos;
        self.position.line = new_position.line;
        self.position.column = new_position.logical_column;
        self.last_movement = Some(action);
    }
}
```

## 4. 文字移動の詳細設計

### 4.1 左右移動アルゴリズム

```rust
impl NavigationSystem {
    /// 右移動（C-f, →）
    pub fn move_char_forward(&mut self, text: &str) -> Result<bool, NavigationError> {
        let chars: Vec<char> = text.chars().collect();

        if self.cursor.char_pos >= chars.len() {
            return Ok(false); // ファイル末尾で停止
        }

        let current_char = chars[self.cursor.char_pos];
        let new_char_pos = self.cursor.char_pos + 1;

        // 改行文字の処理
        if current_char == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 0;
        } else {
            self.cursor.column += 1;
        }

        self.cursor.char_pos = new_char_pos;
        self.extended_cursor.update_with_line_movement(
            self.position_engine.char_pos_to_line_col(text, new_char_pos)?,
            NavigationAction::MoveCharForward
        );

        Ok(true)
    }

    /// 左移動（C-b, ←）
    pub fn move_char_backward(&mut self, text: &str) -> Result<bool, NavigationError> {
        if self.cursor.char_pos == 0 {
            return Ok(false); // ファイル先頭で停止
        }

        let chars: Vec<char> = text.chars().collect();
        let new_char_pos = self.cursor.char_pos - 1;
        let previous_char = chars[new_char_pos];

        // 改行文字の処理（前の行の末尾への移動）
        if previous_char == '\n' {
            if self.cursor.line > 0 {
                self.cursor.line -= 1;
                // 前の行の長さを計算
                let prev_line_length = self.calculate_line_length(text, self.cursor.line);
                self.cursor.column = prev_line_length;
            }
        } else {
            if self.cursor.column > 0 {
                self.cursor.column -= 1;
            }
        }

        self.cursor.char_pos = new_char_pos;
        self.extended_cursor.update_with_line_movement(
            self.position_engine.char_pos_to_line_col(text, new_char_pos)?,
            NavigationAction::MoveCharBackward
        );

        Ok(true)
    }
}
```

### 4.2 UTF-8文字境界での安全な移動

```rust
/// UTF-8安全な文字移動
impl NavigationSystem {
    /// 安全な前進移動
    fn safe_move_char_forward(&mut self, text: &str) -> Result<bool, NavigationError> {
        let bytes = text.as_bytes();
        let mut byte_pos = 0;
        let mut char_count = 0;

        // 現在の文字位置に対応するバイト位置を見つける
        for (pos, _) in text.char_indices() {
            if char_count == self.cursor.char_pos {
                byte_pos = pos;
                break;
            }
            char_count += 1;
        }

        // 次の文字境界を見つける
        if let Some((next_pos, next_char)) = text[byte_pos..].char_indices().nth(1) {
            let new_char_pos = self.cursor.char_pos + 1;

            // Unicode文字の適切な処理
            if self.is_valid_cursor_position(text, new_char_pos) {
                self.update_cursor_for_char_movement(next_char, new_char_pos, text)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// カーソル位置の妥当性検証
    fn is_valid_cursor_position(&self, text: &str, char_pos: usize) -> bool {
        char_pos <= text.chars().count()
    }

    /// 文字移動時のカーソル更新
    fn update_cursor_for_char_movement(&mut self, moved_char: char, new_char_pos: usize, text: &str) -> Result<(), NavigationError> {
        if moved_char == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 0;
        } else {
            self.cursor.column += 1;
        }

        self.cursor.char_pos = new_char_pos;
        Ok(())
    }
}
```

### 4.3 行境界での折り返し処理

```rust
impl NavigationSystem {
    /// 行境界での折り返し設定
    pub fn set_line_wrap_behavior(&mut self, wrap: LineWrapBehavior) {
        self.line_wrap_behavior = wrap;
    }

    /// 行末での右移動処理
    fn handle_line_end_forward_movement(&mut self, text: &str) -> Result<bool, NavigationError> {
        match self.line_wrap_behavior {
            LineWrapBehavior::NoWrap => {
                // 行末で停止
                Ok(false)
            }
            LineWrapBehavior::WrapToNextLine => {
                // 次の行の先頭に移動
                if self.cursor.line < self.count_total_lines(text).saturating_sub(1) {
                    self.cursor.line += 1;
                    self.cursor.column = 0;
                    self.cursor.char_pos = self.position_engine.line_col_to_char_pos(text, self.cursor.line, 0)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// 行頭での左移動処理
    fn handle_line_start_backward_movement(&mut self, text: &str) -> Result<bool, NavigationError> {
        match self.line_wrap_behavior {
            LineWrapBehavior::NoWrap => {
                // 行頭で停止
                Ok(false)
            }
            LineWrapBehavior::WrapToPrevLine => {
                // 前の行の末尾に移動
                if self.cursor.line > 0 {
                    self.cursor.line -= 1;
                    let prev_line_length = self.calculate_line_length(text, self.cursor.line);
                    self.cursor.column = prev_line_length;
                    self.cursor.char_pos = self.position_engine.line_col_to_char_pos(text, self.cursor.line, prev_line_length)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

/// 行境界での折り返し動作設定
#[derive(Debug, Clone, PartialEq)]
pub enum LineWrapBehavior {
    /// 折り返しなし（Emacsデフォルト）
    NoWrap,
    /// 次の行に折り返し
    WrapToNextLine,
    /// 前の行に折り返し
    WrapToPrevLine,
}
```

## 5. 画面表示との統合

### 5.1 ビューポート管理

```rust
/// 画面表示領域管理
pub struct ViewportManager {
    /// 画面サイズ
    screen_size: (u16, u16), // (width, height)
    /// 現在の表示オフセット
    scroll_offset: (usize, usize), // (line_offset, column_offset)
    /// スクロール設定
    scroll_behavior: ScrollBehavior,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollBehavior {
    /// 中央配置の有効性（QA Q23回答）
    pub center_on_move: bool,
    /// スクロールマージン
    pub scroll_margin: usize,
}

impl ViewportManager {
    pub fn new(screen_size: (u16, u16)) -> Self {
        Self {
            screen_size,
            scroll_offset: (0, 0),
            scroll_behavior: ScrollBehavior {
                center_on_move: true, // QA Q23回答
                scroll_margin: 2,
            },
        }
    }

    /// カーソル移動に伴う画面スクロール処理
    pub fn update_viewport_for_cursor(&mut self, cursor_pos: &Position) -> ViewportUpdate {
        let (screen_width, screen_height) = self.screen_size;
        let visible_height = screen_height as usize;
        let visible_width = screen_width as usize;

        // 垂直スクロールの判定
        let vertical_update = self.calculate_vertical_scroll(cursor_pos.line, visible_height);

        // 水平スクロールの判定
        let horizontal_update = self.calculate_horizontal_scroll(cursor_pos.visual_column, visible_width);

        ViewportUpdate {
            new_scroll_offset: (
                vertical_update.unwrap_or(self.scroll_offset.0),
                horizontal_update.unwrap_or(self.scroll_offset.1)
            ),
            needs_redraw: vertical_update.is_some() || horizontal_update.is_some(),
        }
    }

    /// 垂直スクロール計算
    fn calculate_vertical_scroll(&self, cursor_line: usize, visible_height: usize) -> Option<usize> {
        let current_top = self.scroll_offset.0;
        let current_bottom = current_top + visible_height;

        // カーソルが画面外に出た場合
        if cursor_line < current_top || cursor_line >= current_bottom {
            if self.scroll_behavior.center_on_move {
                // QA Q23回答: 中央配置
                Some(cursor_line.saturating_sub(visible_height / 2))
            } else {
                // 最小スクロール
                if cursor_line < current_top {
                    Some(cursor_line.saturating_sub(self.scroll_behavior.scroll_margin))
                } else {
                    Some(cursor_line + self.scroll_behavior.scroll_margin + 1 - visible_height)
                }
            }
        } else {
            None
        }
    }

    /// 水平スクロール計算
    fn calculate_horizontal_scroll(&self, cursor_column: usize, visible_width: usize) -> Option<usize> {
        let current_left = self.scroll_offset.1;
        let current_right = current_left + visible_width;

        if cursor_column < current_left || cursor_column >= current_right {
            if self.scroll_behavior.center_on_move {
                // 中央配置
                Some(cursor_column.saturating_sub(visible_width / 2))
            } else {
                // 最小スクロール
                if cursor_column < current_left {
                    Some(cursor_column.saturating_sub(self.scroll_behavior.scroll_margin))
                } else {
                    Some(cursor_column + self.scroll_behavior.scroll_margin + 1 - visible_width)
                }
            }
        } else {
            None
        }
    }
}

/// ビューポート更新情報
#[derive(Debug, Clone)]
pub struct ViewportUpdate {
    pub new_scroll_offset: (usize, usize),
    pub needs_redraw: bool,
}
```

### 5.2 ratatui統合

```rust
/// ratatuiとの統合インターフェース
pub struct RatatuiIntegration {
    viewport_manager: ViewportManager,
    coordinate_converter: CoordinateConverter,
}

impl RatatuiIntegration {
    /// ratatui座標系への変換
    pub fn buffer_to_screen_coordinates(&self, buffer_pos: &Position) -> Option<(u16, u16)> {
        let (line_offset, col_offset) = self.viewport_manager.scroll_offset;

        // 画面内に表示されているかチェック
        if buffer_pos.line < line_offset {
            return None;
        }

        let screen_line = buffer_pos.line - line_offset;
        let screen_col = buffer_pos.visual_column.saturating_sub(col_offset);

        let (screen_width, screen_height) = self.viewport_manager.screen_size;

        if screen_line >= screen_height as usize || screen_col >= screen_width as usize {
            None
        } else {
            Some((screen_col as u16, screen_line as u16))
        }
    }

    /// 画面座標からバッファ座標への変換
    pub fn screen_to_buffer_coordinates(&self, screen_x: u16, screen_y: u16, text: &str) -> Result<Position, NavigationError> {
        let (line_offset, col_offset) = self.viewport_manager.scroll_offset;

        let buffer_line = line_offset + screen_y as usize;
        let visual_column = col_offset + screen_x as usize;

        // 実際の文字位置を計算
        self.coordinate_converter.visual_to_logical_position(text, buffer_line, visual_column)
    }
}
```

## 6. 境界処理・エラーハンドリング

### 6.1 境界条件の処理

```rust
/// 境界条件の網羅的処理
impl NavigationSystem {
    /// ファイル先頭での処理
    pub fn handle_buffer_start_boundary(&mut self, movement: NavigationAction) -> BoundaryResult {
        match movement {
            NavigationAction::MoveCharBackward |
            NavigationAction::MoveLineUp => {
                // ファイル先頭で停止
                BoundaryResult::Stopped
            }
            NavigationAction::MoveBufferStart => {
                // 既にファイル先頭
                BoundaryResult::AlreadyAtBoundary
            }
            _ => BoundaryResult::Continue
        }
    }

    /// ファイル末尾での処理
    pub fn handle_buffer_end_boundary(&mut self, movement: NavigationAction, text: &str) -> BoundaryResult {
        let total_chars = text.chars().count();

        match movement {
            NavigationAction::MoveCharForward |
            NavigationAction::MoveLineDown => {
                if self.cursor.char_pos >= total_chars {
                    BoundaryResult::Stopped
                } else {
                    BoundaryResult::Continue
                }
            }
            NavigationAction::MoveBufferEnd => {
                if self.cursor.char_pos == total_chars {
                    BoundaryResult::AlreadyAtBoundary
                } else {
                    BoundaryResult::Continue
                }
            }
            _ => BoundaryResult::Continue
        }
    }

    /// 空ファイルでの移動処理
    pub fn handle_empty_file_navigation(&mut self, movement: NavigationAction) -> BoundaryResult {
        match movement {
            NavigationAction::MoveBufferStart |
            NavigationAction::MoveBufferEnd => {
                // カーソルを原点に固定
                self.cursor.char_pos = 0;
                self.cursor.line = 0;
                self.cursor.column = 0;
                BoundaryResult::AlreadyAtBoundary
            }
            _ => BoundaryResult::Stopped
        }
    }
}

/// 境界処理の結果
#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryResult {
    /// 移動継続
    Continue,
    /// 境界で停止
    Stopped,
    /// 既に境界にいる
    AlreadyAtBoundary,
}
```

### 6.2 エラー処理と復旧

```rust
/// ナビゲーションエラー定義
#[derive(Debug, thiserror::Error)]
pub enum NavigationError {
    #[error("Invalid position: {0}")]
    InvalidPosition(usize),

    #[error("Invalid line: {0}")]
    InvalidLine(usize),

    #[error("Invalid column: {0}")]
    InvalidColumn(usize),

    #[error("Text processing error: {0}")]
    TextProcessingError(String),

    #[error("Performance constraint violated: operation took {duration:?}, limit: {limit:?}")]
    PerformanceConstraintViolated {
        duration: std::time::Duration,
        limit: std::time::Duration,
    },

    #[error("Unicode processing error: {0}")]
    UnicodeError(String),
}

/// エラー復旧システム
impl NavigationSystem {
    /// 不正位置からの自動復旧
    pub fn recover_from_invalid_position(&mut self, text: &str) -> Result<(), NavigationError> {
        let total_chars = text.chars().count();

        // カーソル位置の正規化
        if self.cursor.char_pos > total_chars {
            self.cursor.char_pos = total_chars;
        }

        // 行・列情報の再計算
        let corrected_pos = self.position_engine.char_pos_to_line_col(text, self.cursor.char_pos)?;
        self.cursor.line = corrected_pos.line;
        self.cursor.column = corrected_pos.logical_column;

        // キャッシュの無効化
        self.position_engine.invalidate_cache();

        Ok(())
    }

    /// パフォーマンス制約違反の処理
    pub fn handle_performance_violation(&mut self, violation: &NavigationError) -> Result<(), NavigationError> {
        match violation {
            NavigationError::PerformanceConstraintViolated { duration, limit } => {
                // 長い行での性能劣化許容（QA Q22回答）
                if duration.as_millis() <= 10 {
                    // 10ms以内なら許容
                    Ok(())
                } else {
                    // 10msを超える場合は最適化が必要
                    self.enable_performance_optimization_mode();
                    Err(violation.clone())
                }
            }
            _ => Err(violation.clone())
        }
    }

    /// パフォーマンス最適化モードの有効化
    fn enable_performance_optimization_mode(&mut self) {
        // 長い行用の最適化を有効化
        self.position_engine.enable_long_line_optimization();
        self.viewport_manager.enable_lazy_rendering();
    }
}
```

## 7. パフォーマンス要件と最適化

### 7.1 性能目標の実現

```rust
/// パフォーマンス監視システム
pub struct PerformanceMonitor {
    /// 操作時間の測定
    operation_times: HashMap<NavigationAction, Vec<std::time::Duration>>,
    /// 性能制約
    constraints: PerformanceConstraints,
}

#[derive(Debug, Clone)]
pub struct PerformanceConstraints {
    /// 基本移動操作の制限（QA回答）
    pub basic_movement_limit: std::time::Duration,
    /// 長い行での制限（QA Q22回答）
    pub long_line_limit: std::time::Duration,
    /// 行長の閾値
    pub long_line_threshold: usize,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            operation_times: HashMap::new(),
            constraints: PerformanceConstraints {
                basic_movement_limit: std::time::Duration::from_millis(1), // QA要件
                long_line_limit: std::time::Duration::from_millis(10),     // QA Q22回答
                long_line_threshold: 1000,
            },
        }
    }

    /// 操作の性能測定
    pub fn measure_operation<F, T>(&mut self, action: NavigationAction, operation: F) -> Result<T, NavigationError>
    where
        F: FnOnce() -> Result<T, NavigationError>,
    {
        let start = std::time::Instant::now();
        let result = operation()?;
        let duration = start.elapsed();

        // 性能制約のチェック
        self.check_performance_constraint(action, duration)?;

        // 測定結果の記録
        self.operation_times.entry(action).or_insert_with(Vec::new).push(duration);

        Ok(result)
    }

    /// 性能制約のチェック
    fn check_performance_constraint(&self, action: NavigationAction, duration: std::time::Duration) -> Result<(), NavigationError> {
        let limit = match action {
            NavigationAction::MoveCharForward |
            NavigationAction::MoveCharBackward |
            NavigationAction::MoveLineUp |
            NavigationAction::MoveLineDown => self.constraints.basic_movement_limit,
            _ => self.constraints.long_line_limit,
        };

        if duration > limit {
            Err(NavigationError::PerformanceConstraintViolated { duration, limit })
        } else {
            Ok(())
        }
    }
}
```

### 7.2 長い行での最適化（QA Q22対応）

```rust
/// 長い行用の最適化機能
impl PositionCalculator {
    /// 長い行用最適化の有効化
    pub fn enable_long_line_optimization(&mut self) {
        self.long_line_optimization = true;
    }

    /// 最適化された位置計算（長い行用）
    pub fn optimized_char_pos_to_line_col(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        if !self.long_line_optimization {
            return self.char_pos_to_line_col(text, char_pos);
        }

        // 段階的な最適化アプローチ
        if self.estimated_max_line_length(text) > 1000 {
            self.optimized_calculation_for_long_lines(text, char_pos)
        } else {
            self.char_pos_to_line_col(text, char_pos)
        }
    }

    /// 長い行専用の効率的計算
    fn optimized_calculation_for_long_lines(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        // チャンク単位での処理により計算量を削減
        const CHUNK_SIZE: usize = 1000;

        let mut current_pos = 0;
        let mut line = 0;

        for chunk in text.chars().collect::<Vec<_>>().chunks(CHUNK_SIZE) {
            let chunk_end = current_pos + chunk.len();

            if char_pos < chunk_end {
                // 該当チャンク内での詳細計算
                return self.calculate_position_in_chunk(chunk, char_pos - current_pos, line);
            }

            // 改行数をカウント
            line += chunk.iter().filter(|&&ch| ch == '\n').count();
            current_pos = chunk_end;
        }

        Err(NavigationError::InvalidPosition(char_pos))
    }

    fn calculate_position_in_chunk(&self, chunk: &[char], relative_pos: usize, base_line: usize) -> Result<Position, NavigationError> {
        let mut line = base_line;
        let mut column = 0;

        for (i, &ch) in chunk.iter().enumerate() {
            if i == relative_pos {
                break;
            }

            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        let visual_column = Position::logical_to_visual_column(column, &chunk.iter().collect::<String>(), self.tab_width);

        Ok(Position {
            char_pos: relative_pos,
            line,
            visual_column,
            logical_column: column,
        })
    }

    /// 最大行長の推定
    fn estimated_max_line_length(&self, text: &str) -> usize {
        text.lines().map(|line| line.chars().count()).max().unwrap_or(0)
    }
}
```

## 8. テスト仕様

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod navigation_tests {
    use super::*;

    #[test]
    fn test_basic_character_movement() {
        let mut nav_system = NavigationSystem::new();
        let text = "Hello, World!";

        // 右移動テスト
        assert!(nav_system.move_char_forward(text).unwrap());
        assert_eq!(nav_system.cursor.char_pos, 1);
        assert_eq!(nav_system.cursor.column, 1);

        // 左移動テスト
        assert!(nav_system.move_char_backward(text).unwrap());
        assert_eq!(nav_system.cursor.char_pos, 0);
        assert_eq!(nav_system.cursor.column, 0);
    }

    #[test]
    fn test_line_movement_with_different_lengths() {
        let mut nav_system = NavigationSystem::new();
        let text = "Short\nThis is a longer line\nShort";

        // 2行目の中央に移動
        nav_system.cursor = CursorPosition::at(15, 1, 9); // "longer" の 'g'

        // 上移動（短い行への移動）
        assert!(nav_system.move_line_up(text).unwrap());
        assert_eq!(nav_system.cursor.line, 0);
        assert_eq!(nav_system.cursor.column, 5); // 行末にクランプ

        // 下移動（長い行への移動）
        assert!(nav_system.move_line_down(text).unwrap());
        assert_eq!(nav_system.cursor.line, 1);
        assert_eq!(nav_system.cursor.column, 5); // 希望列位置を維持
    }

    #[test]
    fn test_utf8_character_navigation() {
        let mut nav_system = NavigationSystem::new();
        let text = "Hello 🌟 こんにちは 世界";

        // 絵文字を含む移動
        nav_system.cursor.char_pos = 6; // 🌟の直前
        assert!(nav_system.move_char_forward(text).unwrap());
        assert_eq!(nav_system.cursor.char_pos, 7); // 🌟の直後

        // 日本語文字の移動
        nav_system.cursor.char_pos = 8; // こんにちはの直前
        assert!(nav_system.move_char_forward(text).unwrap());
        assert_eq!(nav_system.cursor.char_pos, 9); // 'こ'の直後
    }

    #[test]
    fn test_boundary_conditions() {
        let mut nav_system = NavigationSystem::new();
        let text = "Single line";

        // ファイル先頭での左移動
        assert!(!nav_system.move_char_backward(text).unwrap());
        assert_eq!(nav_system.cursor.char_pos, 0);

        // ファイル末尾への移動
        nav_system.cursor.char_pos = text.chars().count();
        assert!(!nav_system.move_char_forward(text).unwrap());
    }

    #[test]
    fn test_tab_width_calculation() {
        let nav_system = NavigationSystem::new();
        let line_text = "a\tb\tc";

        let visual_col = Position::logical_to_visual_column(3, line_text, 4);
        assert_eq!(visual_col, 9); // a(1) + tab(3) + b(1) + tab(4) = 9
    }
}
```

### 8.2 パフォーマンステスト

```rust
#[cfg(test)]
mod navigation_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cursor_movement_performance() {
        let mut nav_system = NavigationSystem::new();
        let text = "a".repeat(1000);

        let start = Instant::now();
        for _ in 0..100 {
            nav_system.move_char_forward(&text).unwrap();
        }
        let duration = start.elapsed();

        // 100回の移動が1ms未満で完了することを確認
        assert!(duration.as_millis() < 1, "Movement too slow: {:?}", duration);
    }

    #[test]
    fn test_long_line_performance() {
        let mut nav_system = NavigationSystem::new();
        let long_line = "a".repeat(10000);

        let start = Instant::now();
        nav_system.move_char_forward(&long_line).unwrap();
        let duration = start.elapsed();

        // 長い行でも10ms未満で完了（QA Q22回答）
        assert!(duration.as_millis() < 10, "Long line movement too slow: {:?}", duration);
    }

    #[test]
    fn test_position_calculation_performance() {
        let mut nav_system = NavigationSystem::new();
        let multiline_text = "line\n".repeat(1000);

        let start = Instant::now();
        for i in (0..1000).step_by(10) {
            nav_system.position_engine.char_pos_to_line_col(&multiline_text, i * 5).unwrap();
        }
        let duration = start.elapsed();

        // 100回の位置計算が10ms未満で完了
        assert!(duration.as_millis() < 10, "Position calculation too slow: {:?}", duration);
    }
}
```

## 9. 実装フェーズ

### Phase 1: 基本ナビゲーション（1日）
1. `NavigationSystem`の基本構造
2. 文字単位の前後移動
3. 基本的な行移動
4. 境界条件の処理

### Phase 2: 高度な機能（1日）
1. 位置計算エンジンの最適化
2. UTF-8安全性の確保
3. Tab幅考慮の実装
4. 画面表示統合

### Phase 3: 最適化・テスト（1日）
1. パフォーマンス最適化
2. 長い行対応の実装
3. 包括的テスト
4. エラーハンドリングの完成

## 10. 制限事項

### MVPでの制約
- 単語移動（M-f, M-b）は実装済み（`NavigationAction::MoveWordForward` / `MoveWordBackward`）
- 段落移動は未実装
- 複合文字（結合文字）の詳細対応は基本レベル
- 動的なTab幅変更は未対応

### 既知の制限
- 超長い行（>10000文字）での性能制限
- プラットフォーム固有の文字幅差異
- 一部の特殊Unicode文字での表示幅計算の限界

この設計により、MVPに必要なナビゲーション機能を高速かつ安全に実装し、ユーザーに優れた編集体験を提供することができる。
