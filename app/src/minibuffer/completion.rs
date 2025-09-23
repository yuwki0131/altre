//! 補完機能
//!
//! ファイルパス補完とコマンド補完システム

use crate::error::Result;
use crate::file::path::expand_path;
use std::path::{Path, PathBuf};
use std::fs;

/// 補完エンジンのトレイト
pub trait CompletionEngine {
    /// 入力文字列に対する補完候補を取得
    fn complete(&self, input: &str) -> Result<Vec<String>>;

    /// 共通プレフィックスを取得
    fn common_prefix(&self, candidates: &[String]) -> String;

    /// 補完を適用
    fn apply_completion(&self, input: &str, completion: &str) -> String;
}

/// ファイルパス補完エンジン
#[derive(Debug)]
pub struct PathCompletion {
    /// ホームディレクトリの展開を行うか
    expand_home: bool,
    /// 環境変数の展開を行うか
    expand_env: bool,
    /// 隠しファイルを表示するか
    show_hidden: bool,
}

impl PathCompletion {
    /// 新しいパス補完エンジンを作成
    pub fn new() -> Self {
        Self {
            expand_home: true,
            expand_env: true,
            show_hidden: false,
        }
    }

    /// 隠しファイル表示を設定
    pub fn with_hidden_files(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// 入力パスからディレクトリと部分ファイル名を分離
    fn split_path(&self, input: &str) -> Result<(PathBuf, String)> {
        // 元の入力でディレクトリ終端かチェック
        let is_dir_terminator = input.ends_with('/') || input.ends_with('\\');

        let expanded = if self.expand_home || self.expand_env {
            expand_path(input)?
        } else {
            PathBuf::from(input)
        };

        if is_dir_terminator {
            // ディレクトリのみが指定された場合
            Ok((expanded, String::new()))
        } else {
            // ファイル名の一部が含まれている場合
            let parent = expanded.parent().unwrap_or(Path::new(".")).to_path_buf();
            let file_name = expanded
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Ok((parent, file_name))
        }
    }

    /// ディレクトリ内のエントリを取得
    fn get_directory_entries(&self, dir: &Path) -> Result<Vec<String>> {
        if !dir.exists() {
            return Ok(vec![]);
        }

        if !dir.is_dir() {
            return Ok(vec![]);
        }

        let mut entries = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();

            // 隠しファイルのフィルタリング
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }

            // ディレクトリの場合は末尾に / を付加
            if entry.file_type()?.is_dir() {
                entries.push(format!("{}/", file_name));
            } else {
                entries.push(file_name);
            }
        }

        // ソート
        entries.sort();
        Ok(entries)
    }

    /// 部分マッチするエントリをフィルタリング
    fn filter_entries(&self, entries: &[String], prefix: &str) -> Vec<String> {
        if prefix.is_empty() {
            return entries.to_vec();
        }

        entries
            .iter()
            .filter(|entry| entry.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// 絶対パスを相対パスまたは短縮形に変換
    fn format_path(&self, path: &Path, original_input: &str) -> String {
        let path_str = path.to_string_lossy();

        // ホームディレクトリを ~ に置換
        if self.expand_home {
            if let Ok(home) = std::env::var("HOME") {
                if path_str.starts_with(&home) {
                    return path_str.replace(&home, "~");
                }
            }
        }

        // 元の入力が相対パスだった場合は相対パスで返す
        if !original_input.starts_with('/') && !original_input.starts_with('~') {
            if let Ok(current_dir) = std::env::current_dir() {
                if let Ok(relative) = path.strip_prefix(&current_dir) {
                    return relative.to_string_lossy().to_string();
                }
            }
        }

        path_str.to_string()
    }
}

impl Default for PathCompletion {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionEngine for PathCompletion {
    fn complete(&self, input: &str) -> Result<Vec<String>> {
        if input.is_empty() {
            // 空の場合は現在ディレクトリの内容を返す
            let entries = self.get_directory_entries(Path::new("."))?;
            return Ok(entries);
        }

        let (dir_path, file_prefix) = self.split_path(input)?;
        let entries = self.get_directory_entries(&dir_path)?;
        let filtered = self.filter_entries(&entries, &file_prefix);

        // 完全なパスを構築
        let mut results = Vec::new();
        for entry in filtered {
            let full_path = dir_path.join(&entry);
            let formatted = self.format_path(&full_path, input);
            results.push(formatted);
        }

        Ok(results)
    }

    fn common_prefix(&self, candidates: &[String]) -> String {
        if candidates.is_empty() {
            return String::new();
        }

        if candidates.len() == 1 {
            return candidates[0].clone();
        }

        let first = &candidates[0];
        let mut prefix = String::new();

        for (i, ch) in first.chars().enumerate() {
            if candidates.iter().all(|s| s.chars().nth(i) == Some(ch)) {
                prefix.push(ch);
            } else {
                break;
            }
        }

        prefix
    }

    fn apply_completion(&self, input: &str, completion: &str) -> String {
        // 入力の最後のパス区切り以降を補完で置換
        if let Some(last_slash) = input.rfind('/') {
            format!("{}/{}", &input[..last_slash], completion)
        } else {
            completion.to_string()
        }
    }
}

/// コマンド補完エンジン
#[derive(Debug)]
pub struct CommandCompletion {
    commands: Vec<String>,
}

impl CommandCompletion {
    /// 新しいコマンド補完エンジンを作成
    pub fn new() -> Self {
        let mut commands = vec![
            "forward-char".to_string(),
            "backward-char".to_string(),
            "next-line".to_string(),
            "previous-line".to_string(),
            "delete-backward-char".to_string(),
            "delete-char".to_string(),
            "find-file".to_string(),
            "save-buffer".to_string(),
            "save-buffers-kill-terminal".to_string(),
            "quit".to_string(),
        ];
        commands.sort();

        Self { commands }
    }

    /// コマンドを追加
    pub fn add_command(&mut self, command: String) {
        if !self.commands.contains(&command) {
            self.commands.push(command);
            self.commands.sort();
        }
    }

    /// すべてのコマンドを取得
    pub fn all_commands(&self) -> &[String] {
        &self.commands
    }
}

impl Default for CommandCompletion {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionEngine for CommandCompletion {
    fn complete(&self, input: &str) -> Result<Vec<String>> {
        if input.is_empty() {
            return Ok(self.commands.clone());
        }

        let filtered: Vec<String> = self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(input))
            .cloned()
            .collect();

        Ok(filtered)
    }

    fn common_prefix(&self, candidates: &[String]) -> String {
        PathCompletion::new().common_prefix(candidates)
    }

    fn apply_completion(&self, _input: &str, completion: &str) -> String {
        completion.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_path_completion_creation() {
        let completion = PathCompletion::new();
        assert!(completion.expand_home);
        assert!(completion.expand_env);
        assert!(!completion.show_hidden);
    }

    #[test]
    fn test_split_path() {
        let completion = PathCompletion::new();

        let (_dir, file) = completion.split_path("test/file").unwrap();
        assert_eq!(file, "file");
        // dirはtest/fileのparent()結果なので"test"になる

        let (_dir, file) = completion.split_path("test/").unwrap();
        assert_eq!(file, "");
        // ディレクトリ終端の場合は空のファイル名
    }

    #[test]
    fn test_directory_entries() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // テストファイルを作成
        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::create_dir(temp_path.join("subdir")).unwrap();

        let completion = PathCompletion::new();
        let entries = completion.get_directory_entries(temp_path).unwrap();

        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));
        assert!(entries.contains(&"subdir/".to_string()));
    }

    #[test]
    fn test_filter_entries() {
        let completion = PathCompletion::new();
        let entries = vec![
            "file1.txt".to_string(),
            "file2.txt".to_string(),
            "document.pdf".to_string(),
        ];

        let filtered = completion.filter_entries(&entries, "file");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"file1.txt".to_string()));
        assert!(filtered.contains(&"file2.txt".to_string()));

        let all = completion.filter_entries(&entries, "");
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_command_completion() {
        let mut completion = CommandCompletion::new();
        assert!(completion.all_commands().contains(&"forward-char".to_string()));

        let filtered = completion.complete("for").unwrap();
        assert!(filtered.contains(&"forward-char".to_string()));

        completion.add_command("custom-command".to_string());
        assert!(completion.all_commands().contains(&"custom-command".to_string()));
    }

    #[test]
    fn test_common_prefix() {
        let completion = PathCompletion::new();

        let candidates = vec![
            "prefix_file1.txt".to_string(),
            "prefix_file2.txt".to_string(),
        ];
        let prefix = completion.common_prefix(&candidates);
        assert_eq!(prefix, "prefix_file");

        let single = vec!["single.txt".to_string()];
        let single_prefix = completion.common_prefix(&single);
        assert_eq!(single_prefix, "single.txt");

        let empty: Vec<String> = vec![];
        let empty_prefix = completion.common_prefix(&empty);
        assert_eq!(empty_prefix, "");
    }
}
