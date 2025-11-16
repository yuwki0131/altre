//! 編集操作の統合インターフェース
//!
//! 様々な編集操作を統一的に扱うためのインターフェース

use crate::buffer::{CursorPosition, EditOperations as BaseEditOperations};
use crate::error::{EditError, Result};
use std::time::Instant;

/// 拡張編集操作インターフェース
pub trait ExtendedEditOperations: BaseEditOperations {
    /// 高速バッファリング文字入力
    fn buffer_char(&mut self, ch: char) -> Result<()>;

    /// 高速バッファリング文字列入力
    fn buffer_str(&mut self, s: &str) -> Result<()>;

    /// 入力バッファをフラッシュ
    fn flush_buffer(&mut self) -> Result<()>;

    /// UTF-8安全な範囲置換
    fn replace_range_safe(&mut self, start: usize, end: usize, text: &str) -> Result<String>;

    /// 単語単位削除（前方）
    fn delete_word_backward(&mut self) -> Result<String>;

    /// 単語単位削除（後方）
    fn delete_word_forward(&mut self) -> Result<String>;

    /// 行削除
    fn delete_line(&mut self) -> Result<String>;

    /// 行の開始まで削除
    fn delete_to_line_start(&mut self) -> Result<String>;

    /// 行の終端まで削除
    fn delete_to_line_end(&mut self) -> Result<String>;

    /// インデント挿入
    fn insert_indent(&mut self) -> Result<()>;

    /// インデント削除
    fn delete_indent(&mut self) -> Result<bool>;

    /// 複数行一括挿入
    fn insert_lines(&mut self, lines: &[&str]) -> Result<()>;

    /// パフォーマンス最適化された一括操作
    fn batch_operation<F, T>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut Self) -> Result<T>;
}

/// 編集操作のメトリクス
#[derive(Debug, Clone)]
pub struct EditMetrics {
    /// 操作名
    pub operation: String,
    /// 実行時間
    pub duration: std::time::Duration,
    /// 影響を受けた文字数
    pub chars_affected: usize,
    /// 成功したかどうか
    pub success: bool,
    /// エラーメッセージ（失敗時）
    pub error_message: Option<String>,
}

/// 編集操作のコンテキスト
pub struct EditContext {
    /// 現在のカーソル位置
    pub cursor: CursorPosition,
    /// 選択範囲（開始位置、終了位置）
    pub selection: Option<(usize, usize)>,
    /// 編集モード
    pub mode: EditMode,
    /// パフォーマンス監視
    pub metrics_enabled: bool,
    /// 最後の操作メトリクス
    pub last_metrics: Option<EditMetrics>,
}

/// 編集モード
#[derive(Debug, Clone, PartialEq)]
pub enum EditMode {
    /// 通常モード
    Normal,
    /// 挿入モード
    Insert,
    /// 上書きモード
    Overwrite,
    /// 選択モード
    Selection,
}

impl EditContext {
    /// 新しい編集コンテキストを作成
    pub fn new() -> Self {
        Self {
            cursor: CursorPosition::new(),
            selection: None,
            mode: EditMode::Insert,
            metrics_enabled: false,
            last_metrics: None,
        }
    }

    /// メトリクス監視を有効化
    pub fn enable_metrics(&mut self) {
        self.metrics_enabled = true;
    }

    /// メトリクス監視を無効化
    pub fn disable_metrics(&mut self) {
        self.metrics_enabled = false;
    }

    /// 操作メトリクスを記録
    pub fn record_metrics(
        &mut self,
        operation: &str,
        duration: std::time::Duration,
        chars_affected: usize,
        success: bool,
        error: Option<&str>,
    ) {
        if self.metrics_enabled {
            self.last_metrics = Some(EditMetrics {
                operation: operation.to_string(),
                duration,
                chars_affected,
                success,
                error_message: error.map(|e| e.to_string()),
            });
        }
    }

    /// 選択範囲を設定
    pub fn set_selection(&mut self, start: usize, end: usize) {
        self.selection = Some((start.min(end), start.max(end)));
    }

    /// 選択範囲をクリア
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// 選択範囲があるかチェック
    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    /// 選択範囲の長さを取得
    pub fn selection_length(&self) -> usize {
        self.selection.map(|(start, end)| end - start).unwrap_or(0)
    }
}

impl Default for EditContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 操作結果
#[derive(Debug)]
pub struct OperationResult<T> {
    /// 結果値
    pub result: Result<T>,
    /// 実行メトリクス
    pub metrics: Option<EditMetrics>,
}

impl<T> OperationResult<T> {
    /// 成功結果を作成
    pub fn success(value: T, metrics: Option<EditMetrics>) -> Self {
        Self {
            result: Ok(value),
            metrics,
        }
    }

    /// エラー結果を作成
    pub fn error(error: crate::error::AltreError, metrics: Option<EditMetrics>) -> Self {
        Self {
            result: Err(error),
            metrics,
        }
    }

    /// 結果を取得
    pub fn into_result(self) -> Result<T> {
        self.result
    }

    /// メトリクスを取得
    pub fn metrics(&self) -> Option<&EditMetrics> {
        self.metrics.as_ref()
    }
}

/// 編集操作のユーティリティ関数
pub mod utils {
    use super::*;

    /// 単語境界を判定
    pub fn is_word_boundary(ch: char) -> bool {
        ch.is_whitespace() || (ch.is_ascii_punctuation() && ch != '_')
    }

    /// 行の開始位置を見つける
    pub fn find_line_start(text: &str, pos: usize) -> usize {
        let char_indices: Vec<_> = text.char_indices().collect();

        if pos >= char_indices.len() {
            return text.len();
        }

        let byte_pos = char_indices[pos].0;

        // 現在位置から前方に向かって改行を探す
        for (i, &(byte_idx, ch)) in char_indices.iter().enumerate().rev() {
            if byte_idx >= byte_pos {
                continue;
            }
            if ch == '\n' {
                return i + 1;
            }
        }

        0 // ファイルの先頭
    }

    /// 行の終了位置を見つける
    pub fn find_line_end(text: &str, pos: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();

        if pos >= chars.len() {
            return chars.len();
        }

        // 現在位置から後方に向かって改行を探す
        for (i, &ch) in chars.iter().enumerate().skip(pos) {
            if ch == '\n' {
                return i;
            }
        }

        chars.len() // ファイルの終端
    }

    /// 前の単語の開始位置を見つける
    pub fn find_word_start_backward(text: &str, pos: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();

        if pos == 0 || pos > chars.len() {
            return pos;
        }

        let mut i = pos.saturating_sub(1);

        // 空白をスキップ
        while i > 0 && chars[i].is_whitespace() {
            i -= 1;
        }

        // 単語の開始まで戻る
        while i > 0 && !is_word_boundary(chars[i - 1]) {
            i -= 1;
        }

        i
    }

    /// 次の単語の終了位置を見つける
    pub fn find_word_end_forward(text: &str, pos: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();

        if pos >= chars.len() {
            return chars.len();
        }

        let mut i = pos;

        // 空白をスキップ
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // 単語の終了まで進む
        while i < chars.len() && !is_word_boundary(chars[i]) {
            i += 1;
        }

        i
    }

    /// インデントレベルを計算
    pub fn calculate_indent_level(line: &str, tab_width: usize) -> usize {
        let mut level = 0;
        for ch in line.chars() {
            match ch {
                ' ' => level += 1,
                '\t' => level += tab_width,
                _ => break,
            }
        }
        level
    }

    /// インデント文字列を生成
    pub fn generate_indent(level: usize, use_tabs: bool, tab_width: usize) -> String {
        if use_tabs {
            "\t".repeat(level / tab_width) + &" ".repeat(level % tab_width)
        } else {
            " ".repeat(level)
        }
    }

    /// 次のタブストップまでに必要なスペース数を計算
    pub fn spaces_to_next_tab_stop(line: &str, column: usize, tab_width: usize) -> usize {
        if tab_width == 0 {
            return 0;
        }

        let mut visual_col = 0usize;
        for (idx, ch) in line.chars().enumerate() {
            if idx >= column {
                break;
            }

            if ch == '\t' {
                let remainder = visual_col % tab_width;
                visual_col += if remainder == 0 {
                    tab_width
                } else {
                    tab_width - remainder
                };
            } else {
                visual_col += 1;
            }
        }

        let remainder = visual_col % tab_width;
        if remainder == 0 {
            tab_width
        } else {
            tab_width - remainder
        }
    }

    /// UTF-8文字境界での安全な範囲チェック
    pub fn safe_char_range(text: &str, start: usize, end: usize) -> Result<(usize, usize)> {
        let char_count = text.chars().count();

        if start > char_count || end > char_count || start > end {
            return Err(EditError::OutOfBounds(start.max(end)).into());
        }

        Ok((start, end))
    }

    /// パフォーマンス測定ラッパー
    pub fn measure_operation<F, T>(
        operation_name: &str,
        operation: F,
    ) -> (Result<T>, std::time::Duration)
    where
        F: FnOnce() -> Result<T>,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();

        // 1ms超過の警告
        if duration.as_millis() > 1 {
            eprintln!(
                "Performance Warning: {} took {}ms (target: <1ms)",
                operation_name,
                duration.as_millis()
            );
        }

        (result, duration)
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;

    #[test]
    fn test_edit_context_creation() {
        let context = EditContext::new();
        assert_eq!(context.mode, EditMode::Insert);
        assert!(!context.has_selection());
        assert!(!context.metrics_enabled);
    }

    #[test]
    fn test_selection_management() {
        let mut context = EditContext::new();

        context.set_selection(10, 5); // 順序は自動調整される
        assert!(context.has_selection());
        assert_eq!(context.selection, Some((5, 10)));
        assert_eq!(context.selection_length(), 5);

        context.clear_selection();
        assert!(!context.has_selection());
    }

    #[test]
    fn test_metrics_recording() {
        let mut context = EditContext::new();
        context.enable_metrics();

        context.record_metrics(
            "test_op",
            std::time::Duration::from_millis(2),
            5,
            true,
            None,
        );

        let metrics = context.last_metrics.as_ref().unwrap();
        assert_eq!(metrics.operation, "test_op");
        assert_eq!(metrics.chars_affected, 5);
        assert!(metrics.success);
        assert!(metrics.error_message.is_none());
    }

    #[test]
    fn test_word_boundary_detection() {
        assert!(is_word_boundary(' '));
        assert!(is_word_boundary('\t'));
        assert!(is_word_boundary('.'));
        assert!(is_word_boundary(','));
        assert!(!is_word_boundary('a'));
        assert!(!is_word_boundary('1'));
        assert!(!is_word_boundary('_'));
    }

    #[test]
    fn test_line_boundaries() {
        let text = "line1\nline2\nline3";

        assert_eq!(find_line_start(text, 0), 0); // 最初の行
        assert_eq!(find_line_start(text, 7), 6); // 2行目
        assert_eq!(find_line_start(text, 13), 12); // 3行目

        assert_eq!(find_line_end(text, 0), 5); // 最初の行の終端
        assert_eq!(find_line_end(text, 7), 11); // 2行目の終端
        assert_eq!(find_line_end(text, 13), 17); // 3行目の終端（ファイル終端）
    }

    #[test]
    fn test_word_boundaries() {
        let text = "hello world test";

        assert_eq!(find_word_start_backward(text, 10), 6); // "world"の開始
        assert_eq!(find_word_start_backward(text, 5), 0); // "hello"の開始

        assert_eq!(find_word_end_forward(text, 0), 5); // "hello"の終端
        assert_eq!(find_word_end_forward(text, 6), 11); // "world"の終端
    }

    #[test]
    fn test_indent_calculation() {
        assert_eq!(calculate_indent_level("    text", 4), 4);
        assert_eq!(calculate_indent_level("\ttext", 4), 4);
        assert_eq!(calculate_indent_level("\t  text", 4), 6);
        assert_eq!(calculate_indent_level("text", 4), 0);
    }

    #[test]
    fn test_indent_generation() {
        assert_eq!(generate_indent(4, false, 4), "    ");
        assert_eq!(generate_indent(4, true, 4), "\t");
        assert_eq!(generate_indent(6, true, 4), "\t  ");
    }

    #[test]
    fn test_spaces_to_next_tab_stop() {
        assert_eq!(spaces_to_next_tab_stop("", 0, 4), 4);
        assert_eq!(spaces_to_next_tab_stop("abcd", 4, 4), 4);
        assert_eq!(spaces_to_next_tab_stop("abc", 3, 4), 1);
        assert_eq!(spaces_to_next_tab_stop("\t", 1, 4), 4);
        assert_eq!(spaces_to_next_tab_stop("\tab", 3, 4), 2);
        assert_eq!(spaces_to_next_tab_stop("\tab", 2, 4), 3);
        assert_eq!(spaces_to_next_tab_stop("あい", 2, 4), 2);
    }

    #[test]
    fn test_safe_char_range() {
        let text = "hello";

        assert!(safe_char_range(text, 0, 3).is_ok());
        assert!(safe_char_range(text, 0, 5).is_ok());
        assert!(safe_char_range(text, 3, 1).is_err()); // start > end
        assert!(safe_char_range(text, 0, 10).is_err()); // end > length
    }

    #[test]
    fn test_performance_measurement() {
        let (result, duration) = measure_operation("test", || -> Result<i32> {
            std::thread::sleep(std::time::Duration::from_micros(100));
            Ok(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(duration.as_micros() >= 100);
    }
}
