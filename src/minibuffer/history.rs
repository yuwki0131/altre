//! ミニバッファ履歴管理
//!
//! セッション内でのコマンド履歴を管理する（QA.mdの回答：最小限実装）

use std::collections::VecDeque;

/// セッション内履歴の最大保存数
const MAX_HISTORY_SIZE: usize = 100;

/// セッション内でのコマンド履歴管理
#[derive(Debug, Clone)]
pub struct SessionHistory {
    /// 履歴エントリ（新しいものが先頭）
    entries: VecDeque<String>,
    /// 最大保存数
    max_size: usize,
}

impl SessionHistory {
    /// 新しい履歴管理インスタンスを作成
    pub fn new() -> Self {
        Self::with_capacity(MAX_HISTORY_SIZE)
    }

    /// 指定した容量で履歴管理インスタンスを作成
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// エントリを履歴に追加
    pub fn add_entry(&mut self, entry: String) {
        // 空文字列は追加しない
        if entry.is_empty() {
            return;
        }

        // 重複を避けるため、既存のエントリを削除
        self.entries.retain(|e| e != &entry);

        // 先頭に追加
        self.entries.push_front(entry);

        // サイズ制限
        while self.entries.len() > self.max_size {
            self.entries.pop_back();
        }
    }

    /// 指定されたインデックスのエントリを取得
    /// インデックス0が最新、大きいほど古い
    pub fn get_entry(&self, index: usize) -> Option<&String> {
        self.entries.get(index)
    }

    /// 履歴のサイズを取得
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 履歴が空かどうかを判定
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 履歴をクリア
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 履歴の反復子を取得（新しいものから古いものへ）
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.entries.iter()
    }

    /// 指定したパターンにマッチするエントリを検索
    pub fn search(&self, pattern: &str) -> Vec<(usize, &String)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.contains(pattern))
            .collect()
    }

    /// 最後に追加されたエントリを取得
    pub fn last_entry(&self) -> Option<&String> {
        self.entries.front()
    }

    /// 指定されたコマンドタイプの履歴のみを取得
    /// （例：ファイル操作履歴、コマンド実行履歴など）
    pub fn get_entries_by_type(&self, prefix: &str) -> Vec<&String> {
        self.entries
            .iter()
            .filter(|entry| entry.starts_with(prefix))
            .collect()
    }
}

impl Default for SessionHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry() {
        let mut history = SessionHistory::new();

        history.add_entry("command1".to_string());
        history.add_entry("command2".to_string());

        assert_eq!(history.len(), 2);
        assert_eq!(history.get_entry(0), Some(&"command2".to_string()));
        assert_eq!(history.get_entry(1), Some(&"command1".to_string()));
    }

    #[test]
    fn test_duplicate_removal() {
        let mut history = SessionHistory::new();

        history.add_entry("command1".to_string());
        history.add_entry("command2".to_string());
        history.add_entry("command1".to_string()); // 重複

        assert_eq!(history.len(), 2);
        assert_eq!(history.get_entry(0), Some(&"command1".to_string()));
        assert_eq!(history.get_entry(1), Some(&"command2".to_string()));
    }

    #[test]
    fn test_empty_entry_ignored() {
        let mut history = SessionHistory::new();

        history.add_entry("".to_string());
        history.add_entry("command1".to_string());
        history.add_entry("".to_string());

        assert_eq!(history.len(), 1);
        assert_eq!(history.get_entry(0), Some(&"command1".to_string()));
    }

    #[test]
    fn test_capacity_limit() {
        let mut history = SessionHistory::with_capacity(2);

        history.add_entry("command1".to_string());
        history.add_entry("command2".to_string());
        history.add_entry("command3".to_string());

        assert_eq!(history.len(), 2);
        assert_eq!(history.get_entry(0), Some(&"command3".to_string()));
        assert_eq!(history.get_entry(1), Some(&"command2".to_string()));
        assert_eq!(history.get_entry(2), None); // command1 は削除されている
    }

    #[test]
    fn test_search() {
        let mut history = SessionHistory::new();

        history.add_entry("find-file test.txt".to_string());
        history.add_entry("save-buffer".to_string());
        history.add_entry("find-file another.txt".to_string());

        let results = history.search("find-file");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1, &"find-file another.txt".to_string());
        assert_eq!(results[1].1, &"find-file test.txt".to_string());
    }

    #[test]
    fn test_get_entries_by_type() {
        let mut history = SessionHistory::new();

        history.add_entry("find-file test.txt".to_string());
        history.add_entry("save-buffer".to_string());
        history.add_entry("find-file another.txt".to_string());
        history.add_entry("execute-command".to_string());

        let find_files = history.get_entries_by_type("find-file");
        assert_eq!(find_files.len(), 2);

        let saves = history.get_entries_by_type("save");
        assert_eq!(saves.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut history = SessionHistory::new();

        history.add_entry("command1".to_string());
        history.add_entry("command2".to_string());

        assert_eq!(history.len(), 2);

        history.clear();

        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
    }
}