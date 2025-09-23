//! 入力バッファシステム
//!
//! 高速な連続入力に対応するバッファリング機能

use std::time::{Duration, Instant};

/// 高速な連続入力に対応するバッファリング
pub struct InputBuffer {
    /// 入力待ちの文字列
    pending_chars: String,
    /// 最後の入力時刻
    last_input_time: Instant,
    /// バッファリング閾値（1ms）
    buffer_timeout: Duration,
    /// 最大バッファサイズ（メモリ保護）
    max_buffer_size: usize,
}

impl InputBuffer {
    /// 新しい入力バッファを作成
    pub fn new() -> Self {
        Self {
            pending_chars: String::new(),
            last_input_time: Instant::now(),
            buffer_timeout: Duration::from_millis(1),
            max_buffer_size: 1024, // 1KB制限
        }
    }

    /// カスタム設定で入力バッファを作成
    pub fn with_config(timeout_ms: u64, max_size: usize) -> Self {
        Self {
            pending_chars: String::new(),
            last_input_time: Instant::now(),
            buffer_timeout: Duration::from_millis(timeout_ms),
            max_buffer_size: max_size,
        }
    }

    /// 文字を入力バッファに追加
    pub fn add_char(&mut self, ch: char) -> Result<(), InputBufferError> {
        // メモリ保護：バッファサイズ制限チェック
        if self.pending_chars.len() + ch.len_utf8() > self.max_buffer_size {
            return Err(InputBufferError::BufferOverflow { max_size: self.max_buffer_size });
        }

        self.pending_chars.push(ch);
        self.last_input_time = Instant::now();
        Ok(())
    }

    /// 文字列を入力バッファに追加
    pub fn add_str(&mut self, s: &str) -> Result<(), InputBufferError> {
        if self.pending_chars.len() + s.len() > self.max_buffer_size {
            return Err(InputBufferError::BufferOverflow { max_size: self.max_buffer_size });
        }

        self.pending_chars.push_str(s);
        self.last_input_time = Instant::now();
        Ok(())
    }

    /// バッファの内容をフラッシュすべきかチェック
    pub fn should_flush(&self) -> bool {
        !self.pending_chars.is_empty() &&
        self.last_input_time.elapsed() > self.buffer_timeout
    }

    /// バッファの内容を強制的にフラッシュすべきかチェック
    pub fn should_force_flush(&self) -> bool {
        self.pending_chars.len() >= self.max_buffer_size / 2
    }

    /// バッファの内容を取得してクリア
    pub fn flush(&mut self) -> String {
        std::mem::take(&mut self.pending_chars)
    }

    /// バッファが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.pending_chars.is_empty()
    }

    /// 現在のバッファサイズを取得
    pub fn len(&self) -> usize {
        self.pending_chars.len()
    }

    /// バッファ内容を覗き見（消費しない）
    pub fn peek(&self) -> &str {
        &self.pending_chars
    }

    /// バッファをクリア
    pub fn clear(&mut self) {
        self.pending_chars.clear();
    }

    /// 設定を更新
    pub fn update_config(&mut self, timeout_ms: u64, max_size: usize) {
        self.buffer_timeout = Duration::from_millis(timeout_ms);
        self.max_buffer_size = max_size;

        // 現在のバッファが制限を超えている場合は切り詰め
        if self.pending_chars.len() > max_size {
            self.pending_chars.truncate(max_size);
        }
    }

    /// 統計情報を取得
    pub fn stats(&self) -> InputBufferStats {
        InputBufferStats {
            current_size: self.pending_chars.len(),
            max_size: self.max_buffer_size,
            timeout: self.buffer_timeout,
            last_input_age: self.last_input_time.elapsed(),
        }
    }
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// 入力バッファエラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum InputBufferError {
    #[error("Input buffer overflow (max size: {max_size})")]
    BufferOverflow { max_size: usize },

    #[error("Invalid input character: {ch:?}")]
    InvalidChar { ch: char },

    #[error("Buffer operation failed: {message}")]
    OperationFailed { message: String },
}

/// 入力バッファ統計情報
#[derive(Debug, Clone)]
pub struct InputBufferStats {
    /// 現在のバッファサイズ
    pub current_size: usize,
    /// 最大バッファサイズ
    pub max_size: usize,
    /// タイムアウト設定
    pub timeout: Duration,
    /// 最後の入力からの経過時間
    pub last_input_age: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_input_buffer_creation() {
        let buffer = InputBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_add_char() {
        let mut buffer = InputBuffer::new();

        assert!(buffer.add_char('a').is_ok());
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.peek(), "a");
    }

    #[test]
    fn test_add_str() {
        let mut buffer = InputBuffer::new();

        assert!(buffer.add_str("hello").is_ok());
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.peek(), "hello");
    }

    #[test]
    fn test_flush() {
        let mut buffer = InputBuffer::new();
        buffer.add_str("test").unwrap();

        let flushed = buffer.flush();
        assert_eq!(flushed, "test");
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = InputBuffer::with_config(1, 5); // 5バイト制限

        buffer.add_str("hello").unwrap();
        let result = buffer.add_char('!');

        assert!(result.is_err());
        match result {
            Err(InputBufferError::BufferOverflow { .. }) => {},
            _ => panic!("Expected BufferOverflow error"),
        }
    }

    #[test]
    fn test_should_flush_timeout() {
        let mut buffer = InputBuffer::with_config(1, 1024); // 1ms timeout
        buffer.add_char('a').unwrap();

        // すぐにはフラッシュしない
        assert!(!buffer.should_flush());

        // 少し待つ
        thread::sleep(Duration::from_millis(2));
        assert!(buffer.should_flush());
    }

    #[test]
    fn test_force_flush() {
        let mut buffer = InputBuffer::with_config(1000, 10); // 10バイト、1秒タイムアウト

        // バッファの半分以上を埋める
        buffer.add_str("12345").unwrap(); // 5バイト = 50%
        assert!(buffer.should_force_flush());
    }

    #[test]
    fn test_utf8_support() {
        let mut buffer = InputBuffer::new();

        assert!(buffer.add_char('あ').is_ok());
        assert!(buffer.add_str("こんにちは").is_ok());

        let content = buffer.flush();
        assert_eq!(content, "あこんにちは");
    }

    #[test]
    fn test_config_update() {
        let mut buffer = InputBuffer::new();
        buffer.add_str("test").unwrap();

        // より小さな制限に更新
        buffer.update_config(2, 3);

        // バッファが切り詰められているはず
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.peek(), "tes");
    }

    #[test]
    fn test_stats() {
        let mut buffer = InputBuffer::with_config(100, 1024);
        buffer.add_str("hello").unwrap();

        let stats = buffer.stats();
        assert_eq!(stats.current_size, 5);
        assert_eq!(stats.max_size, 1024);
        assert_eq!(stats.timeout, Duration::from_millis(100));
        assert!(stats.last_input_age < Duration::from_millis(10));
    }
}