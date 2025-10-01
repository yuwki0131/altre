//! キルリング実装

const DEFAULT_CAPACITY: usize = 32;

/// Emacs風キルリング。
#[derive(Debug, Default)]
pub struct KillRing {
    entries: std::collections::VecDeque<String>,
    capacity: usize,
}

impl KillRing {
    /// 新しいキルリングを作成
    pub fn new() -> Self {
        Self {
            entries: std::collections::VecDeque::with_capacity(DEFAULT_CAPACITY),
            capacity: DEFAULT_CAPACITY,
        }
    }

    /// 最大保持数を設定
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: std::collections::VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// 文字列をキルリングに追加（空文字は無視）
    pub fn push(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.entries.len() == self.capacity {
            self.entries.pop_back();
        }
        self.entries.push_front(text);
    }

    /// 既存の最新エントリに追記（末尾に結合）
    pub fn append_to_front(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        if let Some(front) = self.entries.front_mut() {
            front.push_str(text);
        } else {
            self.entries.push_front(text.to_string());
        }
    }

    /// 既存の最新エントリの先頭に結合
    pub fn prepend_to_front(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        if let Some(front) = self.entries.front_mut() {
            let mut new_text = String::with_capacity(text.len() + front.len());
            new_text.push_str(text);
            new_text.push_str(front);
            *front = new_text;
        } else {
            self.entries.push_front(text.to_string());
        }
    }

    /// キルリングの先頭を取得
    pub fn front(&self) -> Option<&String> {
        self.entries.front()
    }

    /// 直近のキルをヤンク用に取得
    pub fn yank(&self) -> Option<String> {
        self.entries.front().cloned()
    }

    /// 次のキルエントリへ巡回し、新たなヤンク対象を返す
    pub fn rotate(&mut self) -> Option<String> {
        if self.entries.len() <= 1 {
            return self.entries.front().cloned();
        }

        if let Some(front) = self.entries.pop_front() {
            self.entries.push_back(front);
        }

        self.entries.front().cloned()
    }

    /// エントリをクリア
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// エントリ数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 空かどうか
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_front() {
        let mut ring = KillRing::new();
        ring.push("foo".to_string());
        assert_eq!(ring.front().map(String::as_str), Some("foo"));
    }

    #[test]
    fn append_and_prepend() {
        let mut ring = KillRing::new();
        ring.append_to_front("bar");
        assert_eq!(ring.front().map(String::as_str), Some("bar"));

        ring.append_to_front("baz");
        assert_eq!(ring.front().map(String::as_str), Some("barbaz"));

        ring.prepend_to_front("foo");
        assert_eq!(ring.front().map(String::as_str), Some("foobarbaz"));
    }

    #[test]
    fn rotate_entries() {
        let mut ring = KillRing::new();
        ring.push("first".to_string());
        ring.push("second".to_string());
        ring.push("third".to_string());

        assert_eq!(ring.yank().unwrap(), "third");
        assert_eq!(ring.rotate().unwrap(), "second");
        assert_eq!(ring.rotate().unwrap(), "first");
        assert_eq!(ring.rotate().unwrap(), "third");
    }

    #[test]
    fn clear_resets_entries() {
        let mut ring = KillRing::new();
        ring.push("data".to_string());
        ring.clear();
        assert!(ring.is_empty());
    }
}
