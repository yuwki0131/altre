//! ナビゲーションシステム
//!
//! カーソル移動、位置計算、画面表示統合を管理

use crate::buffer::{CursorPosition, GapBuffer};
use crate::error::{AltreError, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// ナビゲーションエラー
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
        duration: Duration,
        limit: Duration,
    },

    #[error("Unicode processing error: {0}")]
    UnicodeError(String),
}

/// ナビゲーション操作の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// 拡張座標情報
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// 文字位置（UTF-8文字単位、0ベース）
    pub char_pos: usize,
    /// 行番号（0ベース）
    pub line: usize,
    /// 表示列番号（Tab考慮、0ベース）
    pub visual_column: usize,
    /// 論理列番号（文字数、0ベース）
    pub logical_column: usize,
}

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

/// 高性能位置計算エンジン
pub struct PositionCalculator {
    /// 行インデックスキャッシュ
    line_index_cache: Vec<usize>,
    /// キャッシュの有効性
    cache_valid: bool,
    /// Tab幅設定（QA Q21回答: 4スペース）
    tab_width: usize,
    /// 長い行用最適化フラグ
    long_line_optimization: bool,
}

impl PositionCalculator {
    pub fn new() -> Self {
        Self {
            line_index_cache: Vec::new(),
            cache_valid: false,
            tab_width: 4, // QA Q21回答
            long_line_optimization: false,
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

    /// 長い行用最適化の有効化
    pub fn enable_long_line_optimization(&mut self) {
        self.long_line_optimization = true;
    }

    /// 最大行長の推定
    fn estimated_max_line_length(&self, text: &str) -> usize {
        text.lines().map(|line| line.chars().count()).max().unwrap_or(0)
    }
}

impl Default for PositionCalculator {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub fn new() -> Self {
        Self {
            position: CursorPosition::new(),
            preferred_column: None,
            last_movement: None,
        }
    }

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

impl Default for ExtendedCursor {
    fn default() -> Self {
        Self::new()
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

/// パフォーマンス制約
#[derive(Debug, Clone)]
pub struct PerformanceConstraints {
    /// 基本移動操作の制限（QA回答）
    pub basic_movement_limit: Duration,
    /// 長い行での制限（QA Q22回答）
    pub long_line_limit: Duration,
    /// 行長の閾値
    pub long_line_threshold: usize,
}

impl Default for PerformanceConstraints {
    fn default() -> Self {
        Self {
            basic_movement_limit: Duration::from_millis(1), // QA要件
            long_line_limit: Duration::from_millis(10),     // QA Q22回答
            long_line_threshold: 1000,
        }
    }
}

/// パフォーマンス監視システム
pub struct PerformanceMonitor {
    /// 操作時間の測定
    operation_times: HashMap<NavigationAction, Vec<Duration>>,
    /// 性能制約
    constraints: PerformanceConstraints,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            operation_times: HashMap::new(),
            constraints: PerformanceConstraints::default(),
        }
    }

    /// 操作の性能測定
    pub fn measure_operation<F, T>(&mut self, action: NavigationAction, operation: F) -> Result<T, NavigationError>
    where
        F: FnOnce() -> Result<T, NavigationError>,
    {
        let start = Instant::now();
        let result = operation()?;
        let duration = start.elapsed();

        // 性能制約のチェック
        self.check_performance_constraint(action, duration)?;

        // 測定結果の記録
        self.operation_times.entry(action).or_insert_with(Vec::new).push(duration);

        Ok(result)
    }

    /// 性能制約のチェック
    fn check_performance_constraint(&self, action: NavigationAction, duration: Duration) -> Result<(), NavigationError> {
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

    /// 統計情報の取得
    pub fn get_statistics(&self, action: NavigationAction) -> Option<NavigationStatistics> {
        self.operation_times.get(&action).map(|times| {
            let mut sorted_times = times.clone();
            sorted_times.sort();

            let average = Duration::from_nanos(
                sorted_times.iter().map(|d| d.as_nanos()).sum::<u128>() / sorted_times.len() as u128
            );

            NavigationStatistics {
                action,
                sample_count: sorted_times.len(),
                average_duration: average,
                median_duration: sorted_times[sorted_times.len() / 2],
                worst_duration: sorted_times.last().copied().unwrap_or_default(),
            }
        })
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// ナビゲーション統計情報
#[derive(Debug, Clone)]
pub struct NavigationStatistics {
    pub action: NavigationAction,
    pub sample_count: usize,
    pub average_duration: Duration,
    pub median_duration: Duration,
    pub worst_duration: Duration,
}

/// メインナビゲーションシステム
pub struct NavigationSystem {
    /// カーソル位置管理
    cursor: CursorPosition,
    /// 拡張カーソル情報
    extended_cursor: ExtendedCursor,
    /// 位置計算エンジン
    position_engine: PositionCalculator,
    /// 行境界での折り返し動作
    line_wrap_behavior: LineWrapBehavior,
    /// パフォーマンス監視
    performance_monitor: PerformanceMonitor,
}

impl NavigationSystem {
    pub fn new() -> Self {
        Self {
            cursor: CursorPosition::new(),
            extended_cursor: ExtendedCursor::new(),
            position_engine: PositionCalculator::new(),
            line_wrap_behavior: LineWrapBehavior::NoWrap,
            performance_monitor: PerformanceMonitor::new(),
        }
    }

    /// カーソル位置を取得
    pub fn cursor(&self) -> &CursorPosition {
        &self.cursor
    }

    /// 拡張カーソル情報を取得
    pub fn extended_cursor(&self) -> &ExtendedCursor {
        &self.extended_cursor
    }

    /// ナビゲーション操作の実行
    pub fn navigate(&mut self, text: &str, action: NavigationAction) -> Result<bool, NavigationError> {
        self.performance_monitor.measure_operation(action, || {
            match action {
                NavigationAction::MoveCharForward => self.move_char_forward(text),
                NavigationAction::MoveCharBackward => self.move_char_backward(text),
                NavigationAction::MoveLineUp => self.move_line_up(text),
                NavigationAction::MoveLineDown => self.move_line_down(text),
                NavigationAction::MoveLineStart => self.move_line_start(text),
                NavigationAction::MoveLineEnd => self.move_line_end(text),
                NavigationAction::MoveBufferStart => self.move_buffer_start(),
                NavigationAction::MoveBufferEnd => self.move_buffer_end(text),
                _ => {
                    // 将来実装予定の機能
                    Err(NavigationError::TextProcessingError(
                        format!("Unimplemented navigation action: {:?}", action)
                    ))
                }
            }
        })
    }

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
        let new_position = self.position_engine.char_pos_to_line_col(text, new_char_pos)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveCharForward);

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
        let new_position = self.position_engine.char_pos_to_line_col(text, new_char_pos)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveCharBackward);

        Ok(true)
    }

    /// 上移動（C-p, ↑）
    pub fn move_line_up(&mut self, text: &str) -> Result<bool, NavigationError> {
        let current_pos = self.position_engine.char_pos_to_line_col(text, self.cursor.char_pos)?;

        if current_pos.line == 0 {
            return Ok(false); // ファイル先頭で停止
        }

        let target_line = current_pos.line - 1;
        let preferred_column = self.extended_cursor.preferred_column.unwrap_or(current_pos.logical_column);

        let target_char_pos = self.calculate_target_position_for_line_move(
            text,
            target_line,
            preferred_column
        )?;

        self.update_cursor_position(target_char_pos, text)?;
        let new_position = self.position_engine.char_pos_to_line_col(text, target_char_pos)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveLineUp);

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
        let preferred_column = self.extended_cursor.preferred_column.unwrap_or(current_pos.logical_column);

        let target_char_pos = self.calculate_target_position_for_line_move(
            text,
            target_line,
            preferred_column
        )?;

        self.update_cursor_position(target_char_pos, text)?;
        let new_position = self.position_engine.char_pos_to_line_col(text, target_char_pos)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveLineDown);

        Ok(true)
    }

    /// 行頭移動（C-a）
    pub fn move_line_start(&mut self, text: &str) -> Result<bool, NavigationError> {
        let current_pos = self.position_engine.char_pos_to_line_col(text, self.cursor.char_pos)?;
        let target_char_pos = self.position_engine.line_col_to_char_pos(text, current_pos.line, 0)?;

        self.update_cursor_position(target_char_pos, text)?;
        let new_position = self.position_engine.char_pos_to_line_col(text, target_char_pos)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveLineStart);

        Ok(true)
    }

    /// 行末移動（C-e）
    pub fn move_line_end(&mut self, text: &str) -> Result<bool, NavigationError> {
        let current_pos = self.position_engine.char_pos_to_line_col(text, self.cursor.char_pos)?;
        let line_length = self.calculate_line_length(text, current_pos.line);
        let target_char_pos = self.position_engine.line_col_to_char_pos(text, current_pos.line, line_length)?;

        self.update_cursor_position(target_char_pos, text)?;
        let new_position = self.position_engine.char_pos_to_line_col(text, target_char_pos)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveLineEnd);

        Ok(true)
    }

    /// バッファ先頭移動
    pub fn move_buffer_start(&mut self) -> Result<bool, NavigationError> {
        self.cursor.char_pos = 0;
        self.cursor.line = 0;
        self.cursor.column = 0;

        let new_position = Position {
            char_pos: 0,
            line: 0,
            visual_column: 0,
            logical_column: 0,
        };
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveBufferStart);

        Ok(true)
    }

    /// バッファ末尾移動
    pub fn move_buffer_end(&mut self, text: &str) -> Result<bool, NavigationError> {
        let total_chars = text.chars().count();
        let total_lines = self.count_total_lines(text);

        self.cursor.char_pos = total_chars;
        self.cursor.line = total_lines.saturating_sub(1);

        if total_lines > 0 {
            self.cursor.column = self.calculate_line_length(text, self.cursor.line);
        } else {
            self.cursor.column = 0;
        }

        let new_position = self.position_engine.char_pos_to_line_col(text, total_chars)?;
        self.extended_cursor.update_with_line_movement(new_position, NavigationAction::MoveBufferEnd);

        Ok(true)
    }

    /// 行移動時の目標位置計算
    fn calculate_target_position_for_line_move(
        &mut self,
        text: &str,
        target_line: usize,
        preferred_column: usize
    ) -> Result<usize, NavigationError> {
        let target_line_length = self.calculate_line_length(text, target_line);
        let actual_column = preferred_column.min(target_line_length);
        self.position_engine.line_col_to_char_pos(text, target_line, actual_column)
    }

    /// カーソル位置の更新
    fn update_cursor_position(&mut self, new_char_pos: usize, text: &str) -> Result<(), NavigationError> {
        let new_position = self.position_engine.char_pos_to_line_col(text, new_char_pos)?;
        self.cursor.char_pos = new_char_pos;
        self.cursor.line = new_position.line;
        self.cursor.column = new_position.logical_column;
        Ok(())
    }

    /// 指定行の文字数を計算
    fn calculate_line_length(&self, text: &str, line: usize) -> usize {
        text.lines().nth(line).map(|l| l.chars().count()).unwrap_or(0)
    }

    /// 総行数を計算
    fn count_total_lines(&self, text: &str) -> usize {
        if text.is_empty() {
            1
        } else {
            text.lines().count()
        }
    }

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

    /// パフォーマンス統計の取得
    pub fn get_performance_statistics(&self, action: NavigationAction) -> Option<NavigationStatistics> {
        self.performance_monitor.get_statistics(action)
    }

    /// 位置計算エンジンの参照を取得
    pub fn position_engine(&mut self) -> &mut PositionCalculator {
        &mut self.position_engine
    }
}

impl Default for NavigationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
        let line_text = "a\tb\tc";
        let visual_col = Position::logical_to_visual_column(3, line_text, 4);
        assert_eq!(visual_col, 9); // a(1) + tab(3) + b(1) + tab(4) = 9
    }

    #[test]
    fn test_line_start_end_movement() {
        let mut nav_system = NavigationSystem::new();
        let text = "Hello\nWorld\n";

        // 1行目の中央に移動
        nav_system.cursor.char_pos = 2;
        nav_system.cursor.line = 0;
        nav_system.cursor.column = 2;

        // 行頭移動
        assert!(nav_system.move_line_start(text).unwrap());
        assert_eq!(nav_system.cursor.column, 0);

        // 行末移動
        assert!(nav_system.move_line_end(text).unwrap());
        assert_eq!(nav_system.cursor.column, 5); // "Hello"の長さ
    }

    #[test]
    fn test_buffer_start_end_movement() {
        let mut nav_system = NavigationSystem::new();
        let text = "Line 1\nLine 2\nLine 3";

        // 中央に移動
        nav_system.cursor.char_pos = 10;

        // バッファ先頭移動
        assert!(nav_system.move_buffer_start().unwrap());
        assert_eq!(nav_system.cursor.char_pos, 0);
        assert_eq!(nav_system.cursor.line, 0);
        assert_eq!(nav_system.cursor.column, 0);

        // バッファ末尾移動
        assert!(nav_system.move_buffer_end(text).unwrap());
        assert_eq!(nav_system.cursor.char_pos, text.chars().count());
    }

    #[test]
    fn test_position_calculator() {
        let mut calc = PositionCalculator::new();
        let text = "Line 1\nLine 2\nLine 3";

        // 文字位置から行・列への変換
        let pos = calc.char_pos_to_line_col(text, 8).unwrap(); // "Line 2"の"i"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.logical_column, 1);

        // 行・列から文字位置への変換
        let char_pos = calc.line_col_to_char_pos(text, 1, 1).unwrap();
        assert_eq!(char_pos, 8);
    }

    #[test]
    fn test_char_display_width() {
        assert_eq!(Position::char_display_width('a'), 1);
        assert_eq!(Position::char_display_width('あ'), 2);
        assert_eq!(Position::char_display_width('🌟'), 2);
        assert_eq!(Position::char_display_width('\t'), 1); // タブは別途計算
    }
}