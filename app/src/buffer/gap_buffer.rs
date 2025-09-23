//! ギャップバッファ実装
//!
//! 効率的なテキスト編集のためのギャップバッファデータ構造

use crate::error::{AltreError, BufferError, Result};
use std::cmp::Ordering;

const DEFAULT_GAP_CAPACITY: usize = 4096;
const MIN_GAP_RESERVE: usize = 1024;
const MAX_GAP_CAPACITY: usize = 64 * 1024;
const GAP_GROWTH_FACTOR: usize = 2;

#[derive(Debug, Clone)]
struct CharBoundaryCache {
    last_char_pos: usize,
    last_byte_pos: usize,
    line_starts: Vec<usize>,
}

/// ギャップバッファ構造体
///
/// テキスト編集を効率的に行うためのデータ構造
/// カーソル位置付近にギャップ（空き領域）を保持し、
/// 挿入・削除操作を高速化する
#[derive(Debug, Clone)]
pub struct GapBuffer {
    /// 内部バッファ（UTF-8バイト列）
    buffer: Vec<u8>,
    /// ギャップの開始位置（バイト単位、テキストインデックスと同一）
    gap_start: usize,
    /// ギャップの終了位置（排他的、バイト単位）
    gap_end: usize,
    /// 文字境界キャッシュ（最適化用）
    char_cache: Option<CharBoundaryCache>,
}

impl GapBuffer {
    fn compute_line_starts(prefix: &str, suffix: &str) -> Vec<usize> {
        let mut starts = vec![0];
        let mut cumulative = 0usize;
        let total_chars = prefix.chars().count() + suffix.chars().count();

        for ch in prefix.chars().chain(suffix.chars()) {
            cumulative += 1;
            if ch == '\n' && cumulative < total_chars {
                starts.push(cumulative);
            }
        }

        starts
    }

    /// 新しいギャップバッファを作成
    ///
    /// デフォルトで4KBの初期容量を持つ
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_GAP_CAPACITY)
    }

    /// 指定容量で新しいギャップバッファを作成
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize(capacity, 0);

        Self {
            buffer,
            gap_start: 0,
            gap_end: capacity,
            char_cache: None,
        }
    }

    /// 文字列からギャップバッファを作成
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let gap_size = (bytes.len().max(DEFAULT_GAP_CAPACITY) / 4).max(MIN_GAP_RESERVE);
        let total_size = bytes.len() + gap_size;

        let mut buffer = Vec::with_capacity(total_size);
        buffer.extend_from_slice(bytes);
        buffer.resize(total_size, 0);

        Self {
            buffer,
            gap_start: bytes.len(),
            gap_end: total_size,
            char_cache: None,
        }
    }

    fn prefix_bytes(&self) -> &[u8] {
        &self.buffer[..self.gap_start]
    }

    fn suffix_bytes(&self) -> &[u8] {
        &self.buffer[self.gap_end..]
    }

    fn prefix_str(&self) -> &str {
        std::str::from_utf8(self.prefix_bytes()).expect("GapBuffer prefix must be valid UTF-8")
    }

    fn suffix_str(&self) -> &str {
        std::str::from_utf8(self.suffix_bytes()).expect("GapBuffer suffix must be valid UTF-8")
    }

    fn total_text_len(&self) -> usize {
        self.prefix_bytes().len() + self.suffix_bytes().len()
    }

    fn invalidate_cache(&mut self) {
        self.char_cache = None;
    }

    /// 現在のギャップサイズを取得
    pub fn gap_size(&self) -> usize {
        self.gap_end - self.gap_start
    }

    /// 有効な文字数を取得（バイト数ではなく文字数）
    pub fn len_chars(&self) -> usize {
        self.prefix_str().chars().count() + self.suffix_str().chars().count()
    }

    /// 下位互換のためのエイリアス
    pub fn char_len(&self) -> usize {
        self.len_chars()
    }

    /// 有効なバイト数を取得
    pub fn len_bytes(&self) -> usize {
        self.total_text_len()
    }

    /// 下位互換のためのエイリアス
    pub fn byte_len(&self) -> usize {
        self.len_bytes()
    }

    /// 空かどうかを判定
    pub fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }

    /// 全テキストを文字列として取得
    pub fn to_string(&self) -> String {
        let mut result = String::with_capacity(self.total_text_len());
        result.push_str(self.prefix_str());
        result.push_str(self.suffix_str());
        result
    }

    /// 下位互換のためのエイリアス
    pub fn get_text(&self) -> String {
        self.to_string()
    }

    /// 指定範囲のテキストを取得
    pub fn substring(&self, start: usize, end: usize) -> std::result::Result<String, BufferError> {
        if start > end {
            return Err(BufferError::InvalidCursorPosition { position: start });
        }

        let text = self.get_text();
        let char_indices: Vec<_> = text.char_indices().collect();

        if end > char_indices.len() {
            return Err(BufferError::InvalidCursorPosition { position: end });
        }

        let byte_start = if start == 0 { 0 } else { char_indices[start].0 };
        let byte_end = if end == char_indices.len() {
            text.len()
        } else {
            char_indices[end].0
        };

        Ok(text[byte_start..byte_end].to_string())
    }

    /// 下位互換のためのエイリアス
    pub fn get_range(&self, start: usize, end: usize) -> Result<String> {
        self.substring(start, end).map_err(|e| AltreError::Buffer(e))
    }

    /// 指定位置に文字を挿入
    pub fn insert(&mut self, pos: usize, ch: char) -> std::result::Result<(), BufferError> {
        let mut buf = [0; 4];
        let char_bytes = ch.encode_utf8(&mut buf).as_bytes();
        self.insert_bytes_internal(pos, char_bytes)
    }

    /// 下位互換のためのエイリアス
    pub fn insert_char(&mut self, pos: usize, ch: char) -> Result<()> {
        self.insert(pos, ch).map_err(|e| AltreError::Buffer(e))
    }

    /// 指定位置に文字列を挿入
    pub fn insert_str(&mut self, pos: usize, s: &str) -> std::result::Result<(), BufferError> {
        self.insert_bytes_internal(pos, s.as_bytes())
    }

    /// 指定位置のバイト列を挿入（内部用）
    fn insert_bytes_internal(&mut self, char_pos: usize, bytes: &[u8]) -> std::result::Result<(), BufferError> {
        let byte_pos = self.char_to_byte_pos_internal(char_pos)?;
        self.move_gap_to_internal(byte_pos)?;

        // ギャップサイズが不足している場合は拡張
        if self.gap_size() < bytes.len() {
            self.grow_gap_internal(bytes.len())?;
        }

        // バイト列をギャップに挿入
        let gap_pos = self.gap_start;
        self.buffer[gap_pos..gap_pos + bytes.len()].copy_from_slice(bytes);
        self.gap_start += bytes.len();

        self.invalidate_cache();

        Ok(())
    }

    /// 指定位置の文字を削除
    pub fn delete(&mut self, pos: usize) -> std::result::Result<char, BufferError> {
        if pos >= self.len_chars() {
            return Err(BufferError::InvalidCursorPosition { position: pos });
        }

        let byte_pos = self.char_to_byte_pos_internal(pos)?;
        let char_byte_len = self.char_byte_len_at_internal(pos)?;

        // 削除する文字を取得
        let deleted_char = self.char_at(pos)?;

        self.move_gap_to_internal(byte_pos + char_byte_len)?;
        self.gap_start = byte_pos;
        self.invalidate_cache();
        Ok(deleted_char)
    }

    /// 下位互換のためのエイリアス
    pub fn delete_char(&mut self, pos: usize) -> Result<()> {
        self.delete(pos).map(|_| ()).map_err(|e| AltreError::Buffer(e))
    }

    /// 指定範囲を削除
    pub fn delete_range(&mut self, start: usize, end: usize) -> std::result::Result<String, BufferError> {
        if start > end {
            return Err(BufferError::InvalidCursorPosition { position: start });
        }

        let deleted_text = self.substring(start, end)?;

        if deleted_text.is_empty() {
            return Ok(deleted_text);
        }

        let start_byte = self.char_to_byte_pos_internal(start)?;
        let end_byte = self.char_to_byte_pos_internal(end)?;

        self.move_gap_to_internal(end_byte)?;
        self.gap_start = start_byte;
        self.invalidate_cache();

        Ok(deleted_text)
    }

    /// 指定範囲を置換
    pub fn replace_range(&mut self, start: usize, end: usize, s: &str) -> std::result::Result<(), BufferError> {
        self.delete_range(start, end)?;
        self.insert_str(start, s)?;
        Ok(())
    }

    /// 指定位置の文字を取得
    fn char_at(&self, pos: usize) -> std::result::Result<char, BufferError> {
        let prefix = self.prefix_str();
        let prefix_len = prefix.chars().count();

        if pos < prefix_len {
            return prefix
                .chars()
                .nth(pos)
                .ok_or(BufferError::InvalidCursorPosition { position: pos });
        }

        let suffix = self.suffix_str();
        let suffix_pos = pos - prefix_len;

        suffix
            .chars()
            .nth(suffix_pos)
            .ok_or(BufferError::InvalidCursorPosition { position: pos })
    }

    /// 文字位置をバイト位置に変換
    fn char_to_byte_pos_internal(&mut self, char_pos: usize) -> std::result::Result<usize, BufferError> {
        if let Some(cache) = &self.char_cache {
            if cache.last_char_pos == char_pos {
                return Ok(cache.last_byte_pos);
            }
        }

        let total_chars = self.len_chars();
        if char_pos > total_chars {
            return Err(BufferError::InvalidCursorPosition { position: char_pos });
        }

        let prefix = self.prefix_str();
        let prefix_char_count = prefix.chars().count();
        let prefix_byte_len = prefix.len();

        let byte_pos = if char_pos < prefix_char_count {
            // Within prefix
            let mut iter = prefix.char_indices();
            let mut result = prefix_byte_len;
            for (idx, (byte_idx, _)) in iter.by_ref().enumerate() {
                if idx == char_pos {
                    result = byte_idx;
                    break;
                }
            }
            result
        } else if char_pos == prefix_char_count {
            prefix_byte_len
        } else {
            let suffix = self.suffix_str();
            let suffix_target = char_pos - prefix_char_count;
            let suffix_chars = suffix.chars().count();
            if suffix_target > suffix_chars {
                return Err(BufferError::InvalidCursorPosition { position: char_pos });
            }

            if suffix_target == suffix_chars {
                prefix_byte_len + suffix.len()
            } else {
                let mut iter = suffix.char_indices();
                let mut offset = suffix.len();
                for (idx, (byte_idx, _)) in iter.by_ref().enumerate() {
                    if idx == suffix_target {
                        offset = byte_idx;
                        break;
                    }
                }
                prefix_byte_len + offset
            }
        };

        let cache_entry = CharBoundaryCache {
            last_char_pos: char_pos,
            last_byte_pos: byte_pos,
            line_starts: Self::compute_line_starts(prefix, self.suffix_str()),
        };

        self.char_cache = Some(cache_entry);

        Ok(byte_pos)
    }

    /// 指定位置の文字のバイト長を取得
    fn char_byte_len_at_internal(&self, char_pos: usize) -> std::result::Result<usize, BufferError> {
        let ch = self.char_at(char_pos)?;
        Ok(ch.len_utf8())
    }

    /// ギャップ（カーソル）を指定位置に移動
    pub fn move_gap_to(&mut self, byte_pos: usize) -> std::result::Result<(), BufferError> {
        self.move_gap_to_internal(byte_pos)
    }

    /// 現在のギャップ位置を取得（文字単位）
    pub fn gap_position(&self) -> usize {
        // SAFETY: gap_start は常にテキスト境界に揃っているため UTF-8 境界となる
        let text = self.prefix_str();
        text.chars().count()
    }

    /// 行の開始位置（文字単位）のリストを取得
    pub fn line_start_positions(&mut self) -> Vec<usize> {
        if let Some(cache) = &self.char_cache {
            return cache.line_starts.clone();
        }

        // キャッシュが存在しない場合は末尾位置での変換を行い構築する
        let _ = self.char_to_byte_pos_internal(self.len_chars());
        self.char_cache
            .as_ref()
            .map(|cache| cache.line_starts.clone())
            .unwrap_or_else(|| vec![0])
    }

    /// ギャップを指定位置に移動（内部用）
    fn move_gap_to_internal(&mut self, pos: usize) -> std::result::Result<(), BufferError> {
        if pos > self.len_bytes() {
            return Err(BufferError::InvalidCursorPosition { position: pos });
        }

        if pos == self.gap_start {
            return Ok(()); // 既に正しい位置
        }

        if pos < self.gap_start {
            // ギャップを左に移動
            let move_size = self.gap_start - pos;
            let new_gap_end = self.gap_end - move_size;

            // データを右にシフト
            for i in 0..move_size {
                self.buffer[new_gap_end + i] = self.buffer[pos + i];
            }

            self.gap_start = pos;
            self.gap_end = new_gap_end;
        } else {
            // ギャップを右に移動
            let move_size = pos - self.gap_start;
            let new_gap_start = self.gap_start + move_size;

            // データを左にシフト
            for i in 0..move_size {
                self.buffer[self.gap_start + i] = self.buffer[self.gap_end + i];
            }

            self.gap_start = new_gap_start;
            self.gap_end = self.gap_end + move_size;
        }

        Ok(())
    }


    /// バイト位置を文字位置に変換
    #[allow(dead_code)]
    fn byte_to_char_pos_internal(&mut self, byte_pos: usize) -> std::result::Result<usize, BufferError> {
        if let Some(cache) = &self.char_cache {
            if cache.last_byte_pos == byte_pos {
                return Ok(cache.last_char_pos);
            }
        }

        if byte_pos > self.len_bytes() {
            return Err(BufferError::InvalidCursorPosition { position: byte_pos });
        }

        let prefix = self.prefix_str();
        let prefix_byte_len = prefix.len();

        let char_pos = match byte_pos.cmp(&prefix_byte_len) {
            Ordering::Less => {
                let mut count = 0;
                for (idx, _) in prefix.char_indices() {
                    if idx >= byte_pos {
                        break;
                    }
                    count += 1;
                }
                count
            }
            Ordering::Equal => prefix.chars().count(),
            Ordering::Greater => {
                let suffix = self.suffix_str();
                let suffix_offset = byte_pos - prefix_byte_len;
                if suffix_offset > suffix.len() {
                    return Err(BufferError::InvalidCursorPosition { position: byte_pos });
                }

                if suffix_offset == suffix.len() {
                    prefix.chars().count() + suffix.chars().count()
                } else if !suffix.is_char_boundary(suffix_offset) {
                    return Err(BufferError::Utf8Boundary { position: byte_pos });
                } else {
                    let mut count = 0;
                    for (idx, _) in suffix.char_indices() {
                        if idx >= suffix_offset {
                            break;
                        }
                        count += 1;
                    }
                    prefix.chars().count() + count
                }
            }
        };

        let cache_entry = CharBoundaryCache {
            last_char_pos: char_pos,
            last_byte_pos: byte_pos,
            line_starts: Self::compute_line_starts(prefix, self.suffix_str()),
        };
        self.char_cache = Some(cache_entry);

        Ok(char_pos)
    }

    /// ギャップサイズを拡張
    fn grow_gap_internal(&mut self, min_additional: usize) -> std::result::Result<(), BufferError> {
        let current_gap = self.gap_size().max(MIN_GAP_RESERVE);
        let required = min_additional + MIN_GAP_RESERVE;
        let mut new_gap_size = current_gap.saturating_mul(GAP_GROWTH_FACTOR).max(required);
        new_gap_size = new_gap_size.min(MAX_GAP_CAPACITY.max(required));
        let additional_size = new_gap_size - current_gap;

        // 新しいバッファサイズ
        let new_capacity = self.buffer.len() + additional_size;
        let mut new_buffer = Vec::with_capacity(new_capacity);

        // データをコピー
        new_buffer.extend_from_slice(&self.buffer[0..self.gap_start]);
        new_buffer.resize(self.gap_start + new_gap_size, 0);
        new_buffer.extend_from_slice(&self.buffer[self.gap_end..]);

        self.buffer = new_buffer;
        self.gap_end = self.gap_start + new_gap_size;
        self.invalidate_cache();

        Ok(())
    }

}

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prelude::*, prop_oneof};
    use proptest::test_runner::{Config as ProptestConfig, TestCaseError};

    fn char_to_byte_index(s: &str, char_pos: usize) -> usize {
        if char_pos >= s.chars().count() {
            return s.len();
        }
        s.char_indices()
            .nth(char_pos)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len())
    }

    fn small_unicode_string() -> impl Strategy<Value = String> {
        proptest::collection::vec(any::<char>(), 0..64)
            .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    #[derive(Debug, Clone)]
    enum Operation {
        InsertChar { pos: usize, ch: char },
        InsertStr { pos: usize, text: String },
        Delete { pos: usize },
    }

    fn operation_strategy() -> impl Strategy<Value = Operation> {
        let insert_char = (0u16..256u16, any::<char>())
            .prop_map(|(pos, ch)| Operation::InsertChar { pos: pos as usize, ch });
        let insert_str = (0u16..256u16, proptest::collection::vec(any::<char>(), 0..6))
            .prop_map(|(pos, chars)| Operation::InsertStr {
                pos: pos as usize,
                text: chars.into_iter().collect(),
            });
        let delete = (0u16..256u16)
            .prop_map(|pos| Operation::Delete { pos: pos as usize });

        prop_oneof![insert_char, insert_str, delete]
    }

    fn prop_assert_gap_state(buffer: &GapBuffer, expected: &str) -> std::result::Result<(), TestCaseError> {
        prop_assert!(
            buffer.gap_start <= buffer.gap_end,
            "gap start {} exceeds gap end {}",
            buffer.gap_start,
            buffer.gap_end
        );
        prop_assert!(
            buffer.gap_end <= buffer.buffer.len(),
            "gap end {} exceeds buffer length {}",
            buffer.gap_end,
            buffer.buffer.len()
        );
        prop_assert!(
            std::str::from_utf8(&buffer.buffer[..buffer.gap_start]).is_ok(),
            "prefix is not valid UTF-8"
        );
        prop_assert!(
            std::str::from_utf8(&buffer.buffer[buffer.gap_end..]).is_ok(),
            "suffix is not valid UTF-8"
        );
        prop_assert_eq!(
            buffer.len_chars(),
            expected.chars().count(),
            "character length mismatch"
        );
        prop_assert_eq!(buffer.len_bytes(), expected.len(), "byte length mismatch");
        prop_assert_eq!(buffer.get_text(), expected, "text diverged from model");
        Ok(())
    }

    #[test]
    fn test_new_gap_buffer() {
        let gap_buffer = GapBuffer::new();
        assert_eq!(gap_buffer.char_len(), 0);
        assert_eq!(gap_buffer.get_text(), "");
    }

    #[test]
    fn test_from_str() {
        let text = "Hello, world!";
        let gap_buffer = GapBuffer::from_str(text);
        assert_eq!(gap_buffer.get_text(), text);
        assert_eq!(gap_buffer.char_len(), text.chars().count());
    }

    #[test]
    fn test_insert_char() {
        let mut gap_buffer = GapBuffer::new();
        gap_buffer.insert_char(0, 'A').unwrap();
        assert_eq!(gap_buffer.get_text(), "A");
        assert_eq!(gap_buffer.char_len(), 1);
    }

    #[test]
    fn test_insert_char_middle() {
        let mut gap_buffer = GapBuffer::from_str("abcd");
        gap_buffer.insert_char(2, 'X').unwrap();
        assert_eq!(gap_buffer.get_text(), "abXcd");
        assert_eq!(gap_buffer.char_len(), 5);
    }

    #[test]
    fn test_insert_str() {
        let mut gap_buffer = GapBuffer::new();
        gap_buffer.insert_str(0, "Hello").unwrap();
        assert_eq!(gap_buffer.get_text(), "Hello");
        assert_eq!(gap_buffer.char_len(), 5);
    }

    #[test]
    fn test_delete_char() {
        let mut gap_buffer = GapBuffer::from_str("Hello");
        gap_buffer.delete_char(1).unwrap(); // 'e'を削除
        assert_eq!(gap_buffer.get_text(), "Hllo");
        assert_eq!(gap_buffer.char_len(), 4);
    }

    #[test]
    fn test_delete_range() {
        let mut gap_buffer = GapBuffer::from_str("abcdef");
        let deleted = gap_buffer.delete_range(1, 4).unwrap();
        assert_eq!(deleted, "bcd");
        assert_eq!(gap_buffer.get_text(), "aef");
    }

    #[test]
    fn test_line_start_positions() {
        let mut gap_buffer = GapBuffer::from_str("line1\nline2\nline3");
        let lines = gap_buffer.line_start_positions();
        assert_eq!(lines, vec![0, 6, 12]);
    }

    #[test]
    fn test_utf8_support() {
        let mut gap_buffer = GapBuffer::new();
        gap_buffer.insert_str(0, "こんにちは").unwrap();
        assert_eq!(gap_buffer.get_text(), "こんにちは");
        assert_eq!(gap_buffer.char_len(), 5);

        gap_buffer.insert_char(2, '!').unwrap();
        assert_eq!(gap_buffer.get_text(), "こん!にちは");
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

        #[test]
        fn prop_random_operations_preserve_invariants(
            initial in small_unicode_string(),
            ops in proptest::collection::vec(operation_strategy(), 0..24)
        ) {
            let mut buffer = GapBuffer::from_str(&initial);
            let mut model = initial.clone();

            prop_assert_gap_state(&buffer, &model)?;

            for op in ops {
                match op {
                    Operation::InsertChar { pos, ch } => {
                        let insert_pos = pos.min(buffer.len_chars());
                        buffer.insert(insert_pos, ch).unwrap();
                        let byte_idx = char_to_byte_index(&model, insert_pos);
                        model.insert(byte_idx, ch);
                    }
                    Operation::InsertStr { pos, text } => {
                        if text.is_empty() {
                            continue;
                        }
                        let insert_pos = pos.min(buffer.len_chars());
                        buffer.insert_str(insert_pos, &text).unwrap();
                        let byte_idx = char_to_byte_index(&model, insert_pos);
                        model.insert_str(byte_idx, &text);
                    }
                    Operation::Delete { pos } => {
                        if buffer.len_chars() == 0 {
                            continue;
                        }
                        let delete_pos = pos % buffer.len_chars();
                        let expected_char = model.chars().nth(delete_pos).unwrap();
                        let deleted = buffer.delete(delete_pos).unwrap();
                        prop_assert_eq!(deleted, expected_char, "deleted char mismatch");
                        let start = char_to_byte_index(&model, delete_pos);
                        let end = char_to_byte_index(&model, delete_pos + 1);
                        model.replace_range(start..end, "");
                    }
                }

                prop_assert_gap_state(&buffer, &model)?;
            }
        }

        #[test]
        fn prop_multi_byte_insert_is_utf8_safe(
            base in small_unicode_string(),
            ch in any::<char>().prop_filter("multi-byte char", |c| c.len_utf8() > 1),
            pos in 0usize..64
        ) {
            let mut buffer = GapBuffer::from_str(&base);
            let char_len = buffer.len_chars();
            let insert_pos = if char_len == 0 { 0 } else { pos % (char_len + 1) };

            buffer.insert(insert_pos, ch).unwrap();

            let result = buffer.get_text();
            let chars: Vec<char> = result.chars().collect();
            prop_assert_eq!(chars[insert_pos], ch);
            prop_assert!(std::str::from_utf8(result.as_bytes()).is_ok(), "resulting text invalid UTF-8");
        }

        #[test]
        fn prop_out_of_bounds_operations_are_rejected(
            base in small_unicode_string(),
            offset in 1usize..32
        ) {
            let mut buffer = GapBuffer::from_str(&base);
            let invalid_pos = buffer.len_chars() + offset;
            let snapshot = buffer.get_text();

            prop_assert!(buffer.insert(invalid_pos, 'x').is_err());
            prop_assert_eq!(buffer.get_text(), snapshot.as_str(), "buffer mutated after invalid insert");

            if !snapshot.is_empty() {
                prop_assert!(buffer.delete(invalid_pos).is_err());
                prop_assert_eq!(buffer.get_text(), snapshot.as_str(), "buffer mutated after invalid delete");
            }
        }

        #[test]
        fn prop_insert_delete_inverse_round_trips(
            base in small_unicode_string(),
            ch in any::<char>(),
            pos in 0usize..64
        ) {
            let mut buffer = GapBuffer::from_str(&base);
            let char_len = buffer.len_chars();
            let insert_pos = if char_len == 0 { 0 } else { pos % (char_len + 1) };

            buffer.insert(insert_pos, ch).unwrap();
            let deleted = buffer.delete(insert_pos).unwrap();

            prop_assert_eq!(deleted, ch);
            prop_assert_eq!(buffer.get_text(), base);
        }
    }
}
