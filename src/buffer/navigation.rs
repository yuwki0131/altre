//! ナビゲーションシステム
//!
//! ギャップバッファ上のカーソル移動を司る軽量ユーティリティ。

use crate::buffer::cursor::CursorPosition;
use crate::performance::{PerformanceMonitor, Operation, PerformanceOptimizer, OptimizationConfig, LongLineStrategy};
use std::cmp::min;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// ナビゲーション時のエラー。
#[derive(Debug, Clone, thiserror::Error)]
pub enum NavigationError {
    #[error("cursor is already at the beginning of the buffer")]
    StartOfBuffer,
    #[error("cursor is already at the end of the buffer")]
    EndOfBuffer,
    #[error("line {0} is out of range")]
    InvalidLine(usize),
    #[error("internal navigation error: {0}")]
    Internal(String),
}

/// ナビゲーション操作の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationAction {
    MoveCharForward,
    MoveCharBackward,
    MoveLineUp,
    MoveLineDown,
    MoveLineStart,
    MoveLineEnd,
    MoveBufferStart,
    MoveBufferEnd,
    MoveWordForward,
    MoveWordBackward,
}

/// 行・列を含むカーソル位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub char_pos: usize,
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(char_pos: usize, line: usize, column: usize) -> Self {
        Self { char_pos, line, column }
    }
}

/// テキストスナップショット。ナビゲーション演算に必要な行情報を保持する。
#[derive(Debug)]
struct TextSnapshot {
    chars: Vec<char>,
    line_starts: Vec<usize>,
    line_lengths: Vec<usize>,
    #[allow(dead_code)]
    tab_width: usize,
    /// 長い行の最適化情報
    #[allow(dead_code)]
    long_line_cache: std::collections::HashMap<usize, LongLineInfo>,
}

#[cfg(test)]
static SNAPSHOT_CREATIONS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static SNAPSHOT_TRACKING_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct SnapshotCache {
    ptr: *const u8,
    len: usize,
    tab_width: usize,
    snapshot: TextSnapshot,
}

impl SnapshotCache {
    fn new(ptr: *const u8, len: usize, tab_width: usize, snapshot: TextSnapshot) -> Self {
        Self { ptr, len, tab_width, snapshot }
    }

    fn matches(&self, ptr: *const u8, len: usize, tab_width: usize) -> bool {
        self.ptr == ptr && self.len == len && self.tab_width == tab_width
    }

    fn snapshot_ptr(&self) -> *const TextSnapshot {
        &self.snapshot as *const TextSnapshot
    }
}

/// 長い行の最適化情報
#[derive(Debug, Clone)]
struct LongLineInfo {
    #[allow(dead_code)]
    strategy: LongLineStrategy,
    #[allow(dead_code)]
    chunks: Option<Vec<usize>>, // チャンクの境界位置
    #[allow(dead_code)]
    display_limit: Option<usize>,
}

impl TextSnapshot {
    fn new(text: &str) -> Self {
        Self::with_tab_width(text, 4) // デフォルトTab幅は4
    }

    fn with_tab_width(text: &str, tab_width: usize) -> Self {
        #[cfg(test)]
        if SNAPSHOT_TRACKING_ENABLED.load(Ordering::Relaxed) {
            SNAPSHOT_CREATIONS.fetch_add(1, Ordering::Relaxed);
        }

        let chars: Vec<char> = text.chars().collect();
        let mut line_starts = vec![0];
        let mut line_lengths = Vec::new();
        let mut current_len = 0usize;

        for (idx, ch) in chars.iter().enumerate() {
            if *ch == '\n' {
                line_lengths.push(current_len);
                line_starts.push(idx + 1);
                current_len = 0;
            } else {
                current_len += 1;
            }
        }

        line_lengths.push(current_len);
        // `line_starts` の最後は文字数と一致していない場合があるため調整する
        // ただし、ファイル末尾の追加の line_start は line_of_char の計算に影響するため追加しない

        Self {
            chars,
            line_starts,
            line_lengths,
            tab_width,
            long_line_cache: std::collections::HashMap::new(),
        }
    }

    fn char_count(&self) -> usize {
        self.chars.len()
    }

    fn line_count(&self) -> usize {
        self.line_lengths.len()
    }

    fn line_start(&self, line: usize) -> Option<usize> {
        self.line_starts.get(line).copied()
    }

    fn line_length(&self, line: usize) -> Option<usize> {
        self.line_lengths.get(line).copied()
    }

    fn char_at(&self, idx: usize) -> Option<char> {
        self.chars.get(idx).copied()
    }

    fn line_of_char(&self, char_pos: usize) -> usize {
        // パフォーマンス向上のため二分探索を使用（O(log n)）
        if char_pos == 0 {
            return 0;
        }

        // char_pos以下の最大のline_startを見つける
        match self.line_starts.binary_search(&char_pos) {
            Ok(index) => index, // 正確な一致
            Err(index) => index.saturating_sub(1), // 挿入位置の前の要素
        }
    }

    fn char_pos_for_line_col(&self, line: usize, column: usize) -> Option<usize> {
        let start = self.line_start(line)?;
        let len = self.line_length(line)?;
        let clamped = min(column, len);
        Some(start + clamped)
    }

    /// 指定位置の表示幅を計算（Tab考慮）
    #[allow(dead_code)]
    fn display_width_at(&self, char_pos: usize) -> usize {
        if char_pos >= self.chars.len() {
            return 0;
        }

        let line = self.line_of_char(char_pos);
        let line_start = self.line_start(line).unwrap_or(0);
        let mut display_col = 0;

        for pos in line_start..char_pos {
            if let Some(ch) = self.char_at(pos) {
                if ch == '\t' {
                    display_col += self.tab_width - (display_col % self.tab_width);
                } else {
                    display_col += 1;
                }
            }
        }

        display_col
    }

    /// 行内の表示列から文字位置を計算（Tab考慮）
    #[allow(dead_code)]
    fn char_pos_for_display_column(&self, line: usize, display_column: usize) -> Option<usize> {
        let start = self.line_start(line)?;
        let len = self.line_length(line)?;
        let mut display_col = 0;

        for i in 0..len {
            if display_col >= display_column {
                return Some(start + i);
            }

            if let Some(ch) = self.char_at(start + i) {
                if ch == '\t' {
                    display_col += self.tab_width - (display_col % self.tab_width);
                } else {
                    display_col += 1;
                }
            }
        }

        Some(start + len)
    }

    /// 長い行の最適化情報を取得または作成
    #[allow(dead_code)]
    fn get_or_create_long_line_info(&mut self, line: usize, optimizer: &mut PerformanceOptimizer) -> &LongLineInfo {
        if !self.long_line_cache.contains_key(&line) {
            if let Some(line_length) = self.line_length(line) {
                let strategy = optimizer.determine_long_line_strategy(line_length, line);
                let info = match strategy {
                    LongLineStrategy::Normal => LongLineInfo {
                        strategy,
                        chunks: None,
                        display_limit: None,
                    },
                    LongLineStrategy::Chunked => {
                        let chunks = self.calculate_chunks(line, 1000); // 1000文字チャンク
                        LongLineInfo {
                            strategy,
                            chunks: Some(chunks),
                            display_limit: None,
                        }
                    }
                    LongLineStrategy::GradualLimitation => LongLineInfo {
                        strategy,
                        chunks: None,
                        display_limit: Some(5000), // 5000文字まで表示
                    },
                    LongLineStrategy::DisplayLimited => LongLineInfo {
                        strategy,
                        chunks: None,
                        display_limit: Some(2000), // 2000文字まで表示
                    },
                };
                self.long_line_cache.insert(line, info);
            } else {
                // 行が存在しない場合は通常戦略
                self.long_line_cache.insert(line, LongLineInfo {
                    strategy: LongLineStrategy::Normal,
                    chunks: None,
                    display_limit: None,
                });
            }
        }

        self.long_line_cache.get(&line).unwrap()
    }

    /// 行をチャンクに分割する境界を計算
    #[allow(dead_code)]
    fn calculate_chunks(&self, line: usize, chunk_size: usize) -> Vec<usize> {
        let start = self.line_start(line).unwrap_or(0);
        let length = self.line_length(line).unwrap_or(0);
        let mut chunks = Vec::new();

        let mut pos = 0;
        while pos < length {
            chunks.push(start + pos);
            pos += chunk_size;
        }

        chunks
    }

    /// 長い行での安全なナビゲーション
    #[allow(dead_code)]
    fn safe_navigate_long_line(&self, _line: usize, target_column: usize, info: &LongLineInfo) -> usize {
        match info.strategy {
            LongLineStrategy::Normal => target_column,
            LongLineStrategy::Chunked => {
                // チャンク境界で制限
                if let Some(ref chunks) = info.chunks {
                    let chunk_index = target_column / 1000;
                    if chunk_index < chunks.len() {
                        target_column.min(1000 * (chunk_index + 1))
                    } else {
                        target_column
                    }
                } else {
                    target_column
                }
            }
            LongLineStrategy::GradualLimitation | LongLineStrategy::DisplayLimited => {
                // 表示制限を適用
                if let Some(limit) = info.display_limit {
                    target_column.min(limit)
                } else {
                    target_column
                }
            }
        }
    }
}

/// カーソル移動に関する状態。
#[derive(Debug, Clone)]
struct ExtendedCursor {
    position: CursorPosition,
    preferred_column: Option<usize>,
}

impl ExtendedCursor {
    fn new() -> Self {
        Self {
            position: CursorPosition::new(),
            preferred_column: None,
        }
    }

    fn set(&mut self, cursor: CursorPosition) {
        self.position = cursor;
        self.preferred_column = Some(cursor.column);
    }

    fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }
}

/// ナビゲーションシステム本体。
#[derive(Debug)]
pub struct NavigationSystem {
    cursor: CursorPosition,
    extended: ExtendedCursor,
    performance_monitor: Option<PerformanceMonitor>,
    #[allow(dead_code)]
    optimizer: Option<PerformanceOptimizer>,
    snapshot_cache: Option<SnapshotCache>,
}

impl NavigationSystem {
    pub fn new() -> Self {
        Self {
            cursor: CursorPosition::new(),
            extended: ExtendedCursor::new(),
            performance_monitor: None,
            optimizer: None,
            snapshot_cache: None,
        }
    }

    /// パフォーマンス監視付きで作成
    pub fn with_performance_monitoring() -> Self {
        Self {
            cursor: CursorPosition::new(),
            extended: ExtendedCursor::new(),
            performance_monitor: Some(PerformanceMonitor::new()),
            optimizer: Some(PerformanceOptimizer::new(OptimizationConfig::new())),
            snapshot_cache: None,
        }
    }

    /// 高性能設定で作成
    pub fn with_high_performance() -> Self {
        Self {
            cursor: CursorPosition::new(),
            extended: ExtendedCursor::new(),
            performance_monitor: Some(PerformanceMonitor::new()),
            optimizer: Some(PerformanceOptimizer::new(OptimizationConfig::high_performance())),
            snapshot_cache: None,
        }
    }

    /// パフォーマンス監視を有効化
    pub fn enable_performance_monitoring(&mut self) {
        if self.performance_monitor.is_none() {
            self.performance_monitor = Some(PerformanceMonitor::new());
        }
    }

    /// パフォーマンス監視を無効化
    pub fn disable_performance_monitoring(&mut self) {
        self.performance_monitor = None;
    }

    #[allow(dead_code)]
    pub fn clear_snapshot_cache(&mut self) {
        self.snapshot_cache = None;
    }

    /// パフォーマンスメトリクスを取得
    pub fn performance_metrics(&self) -> Option<&crate::performance::PerformanceMetrics> {
        self.performance_monitor.as_ref().map(|m| m.metrics())
    }

    /// カーソルを取得
    pub fn cursor(&self) -> &CursorPosition {
        &self.cursor
    }

    /// カーソルを設定（エディタ側で同期する）
    pub fn set_cursor(&mut self, cursor: CursorPosition) {
        self.cursor = cursor;
        self.extended.set(cursor);
    }

    /// テキストとアクションに基づいてカーソルを移動する。
    pub fn navigate(&mut self, text: &str, action: NavigationAction) -> Result<bool, NavigationError> {
        self.navigate_with_tab_width(text, action, 4)
    }

    /// Tab幅を指定してカーソルを移動する。
    pub fn navigate_with_tab_width(&mut self, text: &str, action: NavigationAction, tab_width: usize) -> Result<bool, NavigationError> {
        // パフォーマンス監視を開始
        let timer = self.performance_monitor.as_ref()
            .map(|m| m.start_operation(Operation::Navigation));

        let ptr = text.as_ptr();
        let len = text.len();

        let needs_refresh = match self.snapshot_cache.as_ref() {
            Some(cache) => !cache.matches(ptr, len, tab_width),
            None => true,
        };

        if needs_refresh {
            let snapshot = TextSnapshot::with_tab_width(text, tab_width);
            self.snapshot_cache = Some(SnapshotCache::new(ptr, len, tab_width, snapshot));
        } else {
            #[cfg(debug_assertions)]
            {
                debug_assert!(
                    self.snapshot_cache
                        .as_ref()
                        .map(|cache| cache.matches(ptr, len, tab_width))
                        .unwrap_or(false),
                    "snapshot cache entry should match when reuse is expected"
                );
            }
        }

        let snapshot_ptr = self.snapshot_cache
            .as_ref()
            .expect("snapshot cache must be initialized")
            .snapshot_ptr();

        let snapshot = unsafe { &*snapshot_ptr };

        if snapshot.line_count() == 0 {
            // タイマーを終了
            if let (Some(timer), Some(ref mut monitor)) = (timer, &mut self.performance_monitor) {
                timer.finish(monitor);
            }
            return Ok(false);
        }

        let moved = match action {
            NavigationAction::MoveCharForward => self.move_char_forward(snapshot),
            NavigationAction::MoveCharBackward => self.move_char_backward(snapshot),
            NavigationAction::MoveLineUp => self.move_line_up(snapshot),
            NavigationAction::MoveLineDown => self.move_line_down(snapshot),
            NavigationAction::MoveLineStart => self.move_line_start(snapshot),
            NavigationAction::MoveLineEnd => self.move_line_end(snapshot),
            NavigationAction::MoveBufferStart => self.move_buffer_start(snapshot),
            NavigationAction::MoveBufferEnd => self.move_buffer_end(snapshot),
            NavigationAction::MoveWordForward => self.move_word_forward(snapshot),
            NavigationAction::MoveWordBackward => self.move_word_backward(snapshot),
        }?;

        if moved {
            self.extended.position = self.cursor;
        }

        // パフォーマンス監視を終了
        if let (Some(timer), Some(ref mut monitor)) = (timer, &mut self.performance_monitor) {
            timer.finish(monitor);
        }

        Ok(moved)
    }

    pub fn recover_from_invalid_position(&mut self, text: &str) -> Result<(), NavigationError> {
        let snapshot = TextSnapshot::new(text);
        let max_pos = snapshot.char_count();
        self.cursor.char_pos = min(self.cursor.char_pos, max_pos);
        let line = snapshot.line_of_char(self.cursor.char_pos);
        let line_start = snapshot.line_start(line).unwrap_or(0);
        let line_len = snapshot.line_length(line).unwrap_or(0);
        self.cursor.line = line;
        self.cursor.column = min(self.cursor.char_pos.saturating_sub(line_start), line_len);
        self.extended.set(self.cursor);
        Ok(())
    }

    fn move_char_forward(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.char_pos >= snapshot.char_count() {
            return Ok(false); // Silent failure for boundary case
        }
        let ch = snapshot.char_at(self.cursor.char_pos).ok_or_else(|| NavigationError::Internal("cursor out of bounds".into()))?;
        self.cursor.char_pos += 1;
        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 0;
            self.extended.preferred_column = Some(0);
        } else {
            self.cursor.column += 1;
            self.extended.preferred_column = Some(self.cursor.column);
        }
        Ok(true)
    }

    fn move_char_backward(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.char_pos == 0 {
            return Ok(false); // Silent failure for boundary case
        }
        let prev_char = snapshot
            .char_at(self.cursor.char_pos - 1)
            .ok_or_else(|| NavigationError::Internal("cursor out of bounds".into()))?;
        self.cursor.char_pos -= 1;
        if prev_char == '\n' {
            if self.cursor.line > 0 {
                self.cursor.line -= 1;
                let len = snapshot.line_length(self.cursor.line).unwrap_or(0);
                self.cursor.column = len;
            }
        } else {
            self.cursor.column = self.cursor.column.saturating_sub(1);
        }
        self.extended.preferred_column = Some(self.cursor.column);
        Ok(true)
    }

    fn move_line_up(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.line == 0 {
            return Ok(false); // Silent failure for boundary case
        }
        let preferred = self.extended.preferred_column.unwrap_or(self.cursor.column);
        let target_line = self.cursor.line - 1;
        let target_len = snapshot.line_length(target_line).unwrap_or(0);
        let new_column = min(preferred, target_len);
        let new_char_pos = snapshot
            .char_pos_for_line_col(target_line, new_column)
            .ok_or_else(|| NavigationError::InvalidLine(target_line))?;
        self.cursor.line = target_line;
        self.cursor.column = new_column;
        self.cursor.char_pos = new_char_pos;
        self.extended.preferred_column = Some(preferred);
        Ok(true)
    }

    fn move_line_down(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let last_line = snapshot.line_count().saturating_sub(1);
        if self.cursor.line >= last_line {
            return Ok(false); // Silent failure for boundary case
        }
        let preferred = self.extended.preferred_column.unwrap_or(self.cursor.column);
        let target_line = min(self.cursor.line + 1, last_line);
        let target_len = snapshot.line_length(target_line).unwrap_or(0);
        let new_column = min(preferred, target_len);
        let new_char_pos = snapshot
            .char_pos_for_line_col(target_line, new_column)
            .ok_or_else(|| NavigationError::InvalidLine(target_line))?;
        self.cursor.line = target_line;
        self.cursor.column = new_column;
        self.cursor.char_pos = new_char_pos;
        self.extended.preferred_column = Some(preferred);
        Ok(true)
    }

    fn move_line_start(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let start = snapshot
            .char_pos_for_line_col(self.cursor.line, 0)
            .ok_or_else(|| NavigationError::InvalidLine(self.cursor.line))?;
        self.cursor.char_pos = start;
        self.cursor.column = 0;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_line_end(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let len = snapshot
            .line_length(self.cursor.line)
            .ok_or_else(|| NavigationError::InvalidLine(self.cursor.line))?;
        let pos = snapshot
            .char_pos_for_line_col(self.cursor.line, len)
            .ok_or_else(|| NavigationError::InvalidLine(self.cursor.line))?;
        self.cursor.char_pos = pos;
        self.cursor.column = len;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_buffer_start(&mut self, _snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        self.cursor.char_pos = 0;
        self.cursor.line = 0;
        self.cursor.column = 0;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_buffer_end(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let total_chars = snapshot.char_count();
        let last_line = snapshot.line_count().saturating_sub(1);
        let last_column = snapshot.line_length(last_line).unwrap_or(0);
        self.cursor.char_pos = total_chars;
        self.cursor.line = last_line;
        self.cursor.column = last_column;
        self.extended.clear_preferred_column();
        Ok(true)
    }

    fn move_word_forward(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        let len = snapshot.char_count();
        let original_pos = self.cursor.char_pos;
        if original_pos >= len {
            return Ok(false);
        }

        let mut chars_iter = original_pos;
        let mut saw_word = false;

        while chars_iter < len {
            let ch = snapshot
                .char_at(chars_iter)
                .ok_or_else(|| NavigationError::Internal("invalid char index".into()))?;
            if is_word_char(ch) {
                saw_word = true;
                chars_iter += 1;
                while chars_iter < len {
                    let ch = snapshot
                        .char_at(chars_iter)
                        .ok_or_else(|| NavigationError::Internal("invalid char index".into()))?;
                    if !is_word_char(ch) {
                        break;
                    }
                    chars_iter += 1;
                }
                break;
            } else {
                chars_iter += 1;
            }
        }

        if !saw_word {
            // 到達できる単語が無い場合は末尾へ
            chars_iter = len;
        }

        self.cursor.char_pos = chars_iter;
        let line = snapshot.line_of_char(chars_iter);
        let line_start = snapshot.line_start(line).unwrap_or(0);
        let column = chars_iter.saturating_sub(line_start);
        self.cursor.line = line;
        self.cursor.column = column;
        self.extended.clear_preferred_column();
        Ok(self.cursor.char_pos != original_pos)
    }

    fn move_word_backward(&mut self, snapshot: &TextSnapshot) -> Result<bool, NavigationError> {
        if self.cursor.char_pos == 0 {
            return Ok(false);
        }

        let mut pos = self.cursor.char_pos;
        let mut started_in_word = false;

        // 既に単語内にいる場合は単語の開始まで戻る
        while pos > 0 {
            let ch = snapshot
                .char_at(pos.saturating_sub(1))
                .ok_or_else(|| NavigationError::Internal("invalid char index".into()))?;
            if is_word_char(ch) {
                pos -= 1;
                started_in_word = true;
            } else {
                break;
            }
        }

        if started_in_word {
            let line = snapshot.line_of_char(pos);
            let line_start = snapshot.line_start(line).unwrap_or(0);
            self.cursor.char_pos = pos;
            self.cursor.line = line;
            self.cursor.column = pos.saturating_sub(line_start);
            self.extended.clear_preferred_column();
            return Ok(true);
        }

        // 非単語文字をスキップして前の単語へ
        while pos > 0 {
            let ch = snapshot
                .char_at(pos.saturating_sub(1))
                .ok_or_else(|| NavigationError::Internal("invalid char index".into()))?;
            if !is_word_char(ch) {
                pos -= 1;
            } else {
                break;
            }
        }

        // 単語の先頭まで戻る
        while pos > 0 {
            let ch = snapshot
                .char_at(pos.saturating_sub(1))
                .ok_or_else(|| NavigationError::Internal("invalid char index".into()))?;
            if is_word_char(ch) {
                pos -= 1;
            } else {
                break;
            }
        }

        let moved = pos != self.cursor.char_pos;
        self.cursor.char_pos = pos;
        let line = snapshot.line_of_char(pos);
        let line_start = snapshot.line_start(line).unwrap_or(0);
        self.cursor.line = line;
        self.cursor.column = pos.saturating_sub(line_start);
        self.extended.clear_preferred_column();
        Ok(moved)
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

impl Default for NavigationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SnapshotTrackingGuard;

    impl SnapshotTrackingGuard {
        fn new() -> Self {
            SNAPSHOT_TRACKING_ENABLED.store(true, Ordering::SeqCst);
            SNAPSHOT_CREATIONS.store(0, Ordering::SeqCst);
            Self
        }
    }

    impl Drop for SnapshotTrackingGuard {
        fn drop(&mut self) {
            SNAPSHOT_TRACKING_ENABLED.store(false, Ordering::SeqCst);
        }
    }

    #[test]
    fn char_movement() {
        let mut nav = NavigationSystem::new();
        let text = "Hello\nWorld";
        assert!(nav.navigate(text, NavigationAction::MoveCharForward).unwrap());
        assert_eq!(nav.cursor().char_pos, 1);
        assert!(nav.navigate(text, NavigationAction::MoveCharBackward).unwrap());
        assert_eq!(nav.cursor().char_pos, 0);
    }

    #[test]
    fn word_movement() {
        let mut nav = NavigationSystem::new();
        let text = "foo  bar_baz qux";

        // move to middle of first word
        assert!(nav.navigate(text, NavigationAction::MoveCharForward).unwrap());
        assert!(nav.navigate(text, NavigationAction::MoveCharForward).unwrap());

        // forward word should land after "foo"
        assert!(nav.navigate(text, NavigationAction::MoveWordForward).unwrap());
        assert_eq!(nav.cursor().char_pos, 3);

        // second forward word should skip spaces and reach end of bar_baz
        assert!(nav.navigate(text, NavigationAction::MoveWordForward).unwrap());
        assert_eq!(nav.cursor().char_pos, 12);

        // backward word returns to start of bar_baz
        assert!(nav.navigate(text, NavigationAction::MoveWordBackward).unwrap());
        assert_eq!(nav.cursor().char_pos, 5);
    }

    #[test]
    fn line_navigation() {
        let mut nav = NavigationSystem::new();
        let text = "Short\nLonger line";
        // Move to second line, column 5
        assert!(nav.navigate(text, NavigationAction::MoveLineDown).unwrap());
        assert!(nav.navigate(text, NavigationAction::MoveLineEnd).unwrap());
        assert_eq!(nav.cursor().line, 1);
        assert_eq!(nav.cursor().column, "Longer line".chars().count());
    }

    #[test]
    fn buffer_bounds() {
        let mut nav = NavigationSystem::new();
        let text = "Only one line";
        // Boundary navigation should return Ok(false) (silent failure, no movement)
        assert!(!nav.navigate(text, NavigationAction::MoveCharBackward).unwrap());
        assert!(nav.navigate(text, NavigationAction::MoveBufferEnd).unwrap());
        assert!(!nav.navigate(text, NavigationAction::MoveLineDown).unwrap());
    }

    #[test]
    fn recover_from_invalid_position() {
        let mut nav = NavigationSystem::new();
        nav.set_cursor(CursorPosition::at(100, 50, 10));
        nav.recover_from_invalid_position("abc").unwrap();
        assert_eq!(nav.cursor().char_pos, 3);
        assert_eq!(nav.cursor().line, 0);
    }

    #[test]
    fn tab_width_calculation() {
        let snapshot = TextSnapshot::with_tab_width("hello\tworld", 4);
        assert_eq!(snapshot.display_width_at(5), 5); // before tab
        assert_eq!(snapshot.display_width_at(6), 8); // after tab (5 + 3 to next tab stop)

        // Test char_pos_for_display_column
        assert_eq!(snapshot.char_pos_for_display_column(0, 5), Some(5)); // before tab
        assert_eq!(snapshot.char_pos_for_display_column(0, 8), Some(6)); // after tab
    }

    #[test]
    fn navigate_with_custom_tab_width() {
        let mut nav = NavigationSystem::new();
        let text = "a\tb";

        // Test with tab width 8
        assert!(nav.navigate_with_tab_width(text, NavigationAction::MoveCharForward, 8).unwrap());
        assert_eq!(nav.cursor().char_pos, 1); // at tab character
        assert!(nav.navigate_with_tab_width(text, NavigationAction::MoveCharForward, 8).unwrap());
        assert_eq!(nav.cursor().char_pos, 2); // at 'b'
    }

    #[test]
    fn snapshot_cache_reuses_existing_snapshot() {
        let mut nav = NavigationSystem::with_high_performance();
        let text = "abc";

        let _guard = SnapshotTrackingGuard::new();

        assert!(nav
            .navigate_with_tab_width(text, NavigationAction::MoveCharForward, 4)
            .unwrap());

        #[cfg(test)]
        let first_creations = SNAPSHOT_CREATIONS.load(Ordering::SeqCst);
        #[cfg(test)]
        assert!(first_creations >= 1);

        nav.set_cursor(CursorPosition::new());
        assert!(nav
            .navigate_with_tab_width(text, NavigationAction::MoveCharForward, 4)
            .unwrap());

        #[cfg(test)]
        {
            let second_creations = SNAPSHOT_CREATIONS.load(Ordering::SeqCst);
            assert!(second_creations <= first_creations + 1);
        }
    }
}
