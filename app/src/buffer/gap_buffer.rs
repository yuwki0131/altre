//! ギャップバッファ実装
//!
//! 効率的なテキスト編集のためのギャップバッファデータ構造

use crate::error::{AltreError, BufferError, Result};

/// ギャップバッファ構造体
///
/// テキスト編集を効率的に行うためのデータ構造
/// カーソル位置付近にギャップ（空き領域）を保持し、
/// 挿入・削除操作を高速化する
#[derive(Debug, Clone)]
pub struct GapBuffer {
    /// 内部バッファ（UTF-8バイト列）
    buffer: Vec<u8>,
    /// ギャップの開始位置（バイト単位）
    gap_start: usize,
    /// ギャップの終了位置（排他的、バイト単位）
    gap_end: usize,
}

impl GapBuffer {
    /// 新しいギャップバッファを作成
    ///
    /// デフォルトで4KBの初期容量を持つ
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    /// 指定容量で新しいギャップバッファを作成
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize(capacity, 0);

        Self {
            buffer,
            gap_start: 0,
            gap_end: capacity,
        }
    }

    /// 文字列からギャップバッファを作成
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let gap_size = (bytes.len().max(4096) / 4).max(1024); // 25%以上、最低1KB
        let total_size = bytes.len() + gap_size;

        let mut buffer = Vec::with_capacity(total_size);
        buffer.extend_from_slice(bytes);
        buffer.resize(total_size, 0);

        Self {
            buffer,
            gap_start: bytes.len(),
            gap_end: total_size,
        }
    }

    /// 現在のギャップサイズを取得
    pub fn gap_size(&self) -> usize {
        self.gap_end - self.gap_start
    }

    /// 有効な文字数を取得（バイト数ではなく文字数）
    pub fn len_chars(&self) -> usize {
        let text = self.get_text();
        text.chars().count()
    }

    /// 下位互換のためのエイリアス
    pub fn char_len(&self) -> usize {
        self.len_chars()
    }

    /// 有効なバイト数を取得
    pub fn len_bytes(&self) -> usize {
        self.buffer.len() - self.gap_size()
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
        let mut result = Vec::new();
        result.extend_from_slice(&self.buffer[0..self.gap_start]);
        result.extend_from_slice(&self.buffer[self.gap_end..]);

        // UTF-8として解釈できない場合は空文字列を返す
        String::from_utf8(result).unwrap_or_default()
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

        for _ in start..end {
            self.delete(start)?;
        }

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
        let text = self.get_text();
        text.chars().nth(pos).ok_or(BufferError::InvalidCursorPosition { position: pos })
    }

    /// 文字位置をバイト位置に変換
    fn char_to_byte_pos_internal(&self, char_pos: usize) -> std::result::Result<usize, BufferError> {
        let text = self.get_text();
        let char_indices: Vec<_> = text.char_indices().collect();

        if char_pos > char_indices.len() {
            return Err(BufferError::InvalidCursorPosition { position: char_pos });
        }

        if char_pos == char_indices.len() {
            Ok(text.len())
        } else {
            Ok(char_indices[char_pos].0)
        }
    }

    /// 指定位置の文字のバイト長を取得
    fn char_byte_len_at_internal(&self, char_pos: usize) -> std::result::Result<usize, BufferError> {
        let text = self.get_text();
        let chars: Vec<char> = text.chars().collect();

        if char_pos >= chars.len() {
            return Err(BufferError::InvalidCursorPosition { position: char_pos });
        }

        Ok(chars[char_pos].len_utf8())
    }

    /// ギャップ（カーソル）を指定位置に移動
    pub fn move_gap_to(&mut self, byte_pos: usize) -> std::result::Result<(), BufferError> {
        self.move_gap_to_internal(byte_pos)
    }

    /// 現在のギャップ位置を取得（文字単位）
    pub fn gap_position(&self) -> usize {
        self.byte_to_char_pos_internal(self.gap_start).unwrap_or(0)
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
    fn byte_to_char_pos_internal(&self, byte_pos: usize) -> std::result::Result<usize, BufferError> {
        let text = self.get_text();
        let bytes = text.as_bytes();

        if byte_pos > bytes.len() {
            return Err(BufferError::InvalidCursorPosition { position: byte_pos });
        }

        let prefix = &bytes[0..byte_pos];
        match std::str::from_utf8(prefix) {
            Ok(s) => Ok(s.chars().count()),
            Err(_) => Err(BufferError::Utf8Boundary { position: byte_pos }),
        }
    }

    /// ギャップサイズを拡張
    fn grow_gap_internal(&mut self, min_additional: usize) -> std::result::Result<(), BufferError> {
        let current_gap = self.gap_size();
        let new_gap_size = (current_gap * 2).max(min_additional + 1024).min(65536);
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
    fn test_utf8_support() {
        let mut gap_buffer = GapBuffer::new();
        gap_buffer.insert_str(0, "こんにちは").unwrap();
        assert_eq!(gap_buffer.get_text(), "こんにちは");
        assert_eq!(gap_buffer.char_len(), 5);

        gap_buffer.insert_char(2, '!').unwrap();
        assert_eq!(gap_buffer.get_text(), "こん!にちは");
    }
}
