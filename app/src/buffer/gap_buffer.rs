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
    use proptest::prelude::*;

    fn char_to_byte_index(s: &str, char_pos: usize) -> usize {
        if char_pos >= s.chars().count() {
            return s.len();
        }
        s.char_indices()
            .nth(char_pos)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len())
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
        #[test]
        fn prop_matches_string_model(initial_text in "[ -~ぁ-んァ-ヶー一-龠０-９]*", ops in prop::collection::vec(any::<(u8, String)>(), 0..20)) {
            let mut gap = GapBuffer::from_str(&initial_text);
            let mut model = initial_text;

            for (selector, payload) in ops {
                let len = gap.len_chars();
                if len == 0 {
                    let insert_pos = 0usize;
                    let chs: Vec<char> = payload.chars().collect();
                    if chs.is_empty() {
                        continue;
                    }
                    let insert_str: String = chs.into_iter().collect();
                    gap.insert_str(insert_pos, &insert_str).unwrap();
                    model.insert_str(insert_pos, &insert_str);
                    continue;
                }

                match selector % 3 {
                    0 => {
                        // Insert a single character if payload not empty
                        if let Some(ch) = payload.chars().next() {
                            let pos = (selector as usize) % (len + 1);
                            gap.insert(pos, ch).unwrap();
                            let byte_idx = char_to_byte_index(&model, pos);
                            model.insert(byte_idx, ch);
                        }
                    }
                    1 => {
                        // Insert string (limited length)
                        if !payload.is_empty() {
                            let pos = (selector as usize) % (len + 1);
                            let snippet: String = payload.chars().take(4).collect();
                            gap.insert_str(pos, &snippet).unwrap();
                            let byte_idx = char_to_byte_index(&model, pos);
                            model.insert_str(byte_idx, &snippet);
                        }
                    }
                    _ => {
                        if len == 0 {
                            continue;
                        }
                        let pos = (selector as usize) % len;
                        gap.delete(pos).unwrap();
                        let start = char_to_byte_index(&model, pos);
                        let end = char_to_byte_index(&model, pos + 1);
                        model.replace_range(start..end, "");
                    }
                }
            }

            prop_assert_eq!(gap.get_text(), model);
        }
    }
}
