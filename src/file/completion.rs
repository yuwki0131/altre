//! パス補完システム
//!
//! ファイルパス補完とミニバッファ統合機能

use crate::error::{AltreError, FileError, Result};
use std::path::{Path, PathBuf};

/// パス補完エンジン
pub struct PathCompletion {
    max_candidates: usize,
    show_hidden: bool,
}

impl PathCompletion {
    pub fn new() -> Self {
        Self {
            max_candidates: 50, // QA Q8の回答
            show_hidden: false,
        }
    }

    /// パス補完を実行
    pub fn complete_path(&self, input: &str) -> Result<CompletionResult> {
        let expanded_path = self.expand_input_path(input)?;
        let (directory, partial_name) = self.split_path_for_completion(&expanded_path);

        if !directory.exists() {
            return Ok(CompletionResult {
                candidates: Vec::new(),
                common_prefix: String::new(),
                is_directory_completion: false,
            });
        }

        let candidates = self.scan_directory(&directory, &partial_name)?;

        Ok(CompletionResult {
            candidates: candidates
                .clone()
                .into_iter()
                .take(self.max_candidates)
                .collect(),
            common_prefix: self.find_common_prefix(&candidates),
            is_directory_completion: directory.is_dir(),
        })
    }

    /// 入力パスを展開
    fn expand_input_path(&self, input: &str) -> Result<PathBuf> {
        // ~/ 展開
        if input.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join(&input[2..]))
            } else {
                Err(AltreError::File(FileError::InvalidPath {
                    path: input.to_string(),
                }))
            }
        } else if input.starts_with('/') {
            Ok(PathBuf::from(input))
        } else {
            Ok(std::env::current_dir()
                .map_err(|e| {
                    AltreError::File(FileError::Io {
                        message: e.to_string(),
                    })
                })?
                .join(input))
        }
    }

    /// パスを補完用に分割
    fn split_path_for_completion(&self, path: &Path) -> (PathBuf, String) {
        if path.is_dir() && path.to_string_lossy().ends_with('/') {
            (path.to_path_buf(), String::new())
        } else {
            let parent = path
                .parent()
                .unwrap_or_else(|| Path::new("/"))
                .to_path_buf();
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            (parent, filename)
        }
    }

    /// ディレクトリ内容をスキャン
    fn scan_directory(&self, dir: &Path, partial: &str) -> Result<Vec<String>> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            AltreError::File(FileError::Io {
                message: e.to_string(),
            })
        })?;

        let mut candidates = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| {
                AltreError::File(FileError::Io {
                    message: e.to_string(),
                })
            })?;
            let filename = entry.file_name().to_string_lossy().to_string();

            // 隠しファイルフィルタ
            if !self.show_hidden && filename.starts_with('.') && !partial.starts_with('.') {
                continue;
            }

            // 部分マッチフィルタ
            if !filename.starts_with(partial) {
                continue;
            }

            // ディレクトリには / サフィックス追加
            let display_name = if entry
                .file_type()
                .map_err(|e| {
                    AltreError::File(FileError::Io {
                        message: e.to_string(),
                    })
                })?
                .is_dir()
            {
                format!("{}/", filename)
            } else {
                filename
            };

            candidates.push(display_name);
        }

        // ソート
        candidates.sort();

        Ok(candidates)
    }

    /// 共通プレフィックスを検索
    fn find_common_prefix(&self, candidates: &[String]) -> String {
        if candidates.is_empty() {
            return String::new();
        }

        if candidates.len() == 1 {
            return candidates[0].clone();
        }

        let first = &candidates[0];
        let mut common_len = 0;

        for (i, ch) in first.char_indices() {
            if candidates
                .iter()
                .all(|candidate| candidate.chars().nth(i).map_or(false, |c| c == ch))
            {
                common_len = i + ch.len_utf8();
            } else {
                break;
            }
        }

        first[..common_len].to_string()
    }

    /// 隠しファイル表示設定
    pub fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden = show;
    }

    /// 最大候補数設定
    pub fn set_max_candidates(&mut self, max: usize) {
        self.max_candidates = max;
    }
}

impl Default for PathCompletion {
    fn default() -> Self {
        Self::new()
    }
}

/// 補完結果
pub struct CompletionResult {
    pub candidates: Vec<String>,
    pub common_prefix: String,
    pub is_directory_completion: bool,
}

impl CompletionResult {
    /// 補完候補が空かどうか
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// 単一候補かどうか
    pub fn is_single_match(&self) -> bool {
        self.candidates.len() == 1
    }

    /// 完全マッチかどうか
    pub fn is_exact_match(&self, input: &str) -> bool {
        self.candidates.len() == 1
            && self.candidates[0].trim_end_matches('/') == input.trim_end_matches('/')
    }
}

/// 補完表示システム
pub struct CompletionDisplay {
    max_display_candidates: usize,
    selected_index: Option<usize>,
}

impl CompletionDisplay {
    pub fn new() -> Self {
        Self {
            max_display_candidates: 10, // ミニバッファ表示制限
            selected_index: None,
        }
    }

    /// 補完テキストをフォーマット
    pub fn format_completion_text(&self, result: &CompletionResult) -> Vec<String> {
        let mut lines = Vec::new();

        if result.is_empty() {
            lines.push("[No matches]".to_string());
            return lines;
        }

        // 候補数表示
        if result.candidates.len() > self.max_display_candidates {
            lines.push(format!(
                "[{} candidates (showing {})]",
                result.candidates.len(),
                self.max_display_candidates
            ));
        }

        // 候補一覧
        for (i, candidate) in result
            .candidates
            .iter()
            .take(self.max_display_candidates)
            .enumerate()
        {
            let prefix = if Some(i) == self.selected_index {
                "► "
            } else {
                "  "
            };
            lines.push(format!("{}{}", prefix, candidate));
        }

        lines
    }

    /// 次の候補を選択
    pub fn select_next(&mut self, total_candidates: usize) {
        if total_candidates == 0 {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(i) => Some((i + 1) % total_candidates.min(self.max_display_candidates)),
            None => Some(0),
        };
    }

    /// 前の候補を選択
    pub fn select_previous(&mut self, total_candidates: usize) {
        if total_candidates == 0 {
            return;
        }

        let display_count = total_candidates.min(self.max_display_candidates);
        self.selected_index = match self.selected_index {
            Some(i) => Some(if i == 0 { display_count - 1 } else { i - 1 }),
            None => Some(display_count - 1),
        };
    }

    /// 選択中の候補を取得
    pub fn get_selected_candidate<'a>(&self, candidates: &'a [String]) -> Option<&'a String> {
        self.selected_index.and_then(|i| candidates.get(i))
    }

    /// 選択をリセット
    pub fn reset_selection(&mut self) {
        self.selected_index = None;
    }

    /// 最初の候補を自動選択
    pub fn auto_select_first(&mut self, total_candidates: usize) {
        if total_candidates > 0 {
            self.selected_index = Some(0);
        }
    }
}

impl Default for CompletionDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// パス展開ユーティリティ
pub struct PathExpander;

impl PathExpander {
    /// チルダ展開
    pub fn expand_tilde(path: &str) -> Result<String> {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join(&path[2..]).to_string_lossy().to_string())
            } else {
                Err(AltreError::File(FileError::InvalidPath {
                    path: "Cannot determine home directory".to_string(),
                }))
            }
        } else if path == "~" {
            if let Some(home) = dirs::home_dir() {
                Ok(home.to_string_lossy().to_string())
            } else {
                Err(AltreError::File(FileError::InvalidPath {
                    path: "Cannot determine home directory".to_string(),
                }))
            }
        } else {
            Ok(path.to_string())
        }
    }

    /// 環境変数展開
    pub fn expand_env_vars(path: &str) -> String {
        // 簡易的な環境変数展開
        if path.contains('$') {
            shellexpand::env(path)
                .unwrap_or_else(|_| path.into())
                .to_string()
        } else {
            path.to_string()
        }
    }

    /// 完全なパス展開
    pub fn expand_full(path: &str) -> Result<String> {
        let tilde_expanded = Self::expand_tilde(path)?;
        let env_expanded = Self::expand_env_vars(&tilde_expanded);
        Ok(env_expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_path_completion_basic() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // テストファイル作成
        fs::write(temp_path.join("test1.txt"), "").unwrap();
        fs::write(temp_path.join("test2.txt"), "").unwrap();
        fs::write(temp_path.join("other.txt"), "").unwrap();

        let completion = PathCompletion::new();
        let input = format!("{}/te", temp_path.display());
        let result = completion.complete_path(&input).unwrap();

        assert_eq!(result.candidates.len(), 2);
        assert!(result.candidates.iter().any(|c| c.contains("test1.txt")));
        assert!(result.candidates.iter().any(|c| c.contains("test2.txt")));
        assert!(!result.candidates.iter().any(|c| c.contains("other.txt")));
    }

    #[test]
    fn test_path_completion_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // ディレクトリ作成
        fs::create_dir(temp_path.join("dir1")).unwrap();
        fs::create_dir(temp_path.join("dir2")).unwrap();

        let completion = PathCompletion::new();
        let input = format!("{}/dir", temp_path.display());
        let result = completion.complete_path(&input).unwrap();

        assert_eq!(result.candidates.len(), 2);
        assert!(result.candidates.iter().any(|c| c == "dir1/"));
        assert!(result.candidates.iter().any(|c| c == "dir2/"));
    }

    #[test]
    fn test_completion_display() {
        let mut display = CompletionDisplay::new();
        let result = CompletionResult {
            candidates: vec![
                "file1.txt".to_string(),
                "file2.txt".to_string(),
                "file3.txt".to_string(),
            ],
            common_prefix: "file".to_string(),
            is_directory_completion: false,
        };

        let formatted = display.format_completion_text(&result);
        assert_eq!(formatted.len(), 3);
        assert!(formatted[0].contains("file1.txt"));

        // 選択テスト
        display.select_next(3);
        let selected = display.get_selected_candidate(&result.candidates);
        assert_eq!(selected, Some(&"file1.txt".to_string()));

        display.select_next(3);
        let selected = display.get_selected_candidate(&result.candidates);
        assert_eq!(selected, Some(&"file2.txt".to_string()));
    }

    #[test]
    fn test_path_expander() {
        // 環境変数展開テスト
        std::env::set_var("TEST_VAR", "test_value");
        let expanded = PathExpander::expand_env_vars("$TEST_VAR/file.txt");
        assert_eq!(expanded, "test_value/file.txt");

        // チルダ展開テスト（ホームディレクトリが利用可能な場合）
        if let Ok(expanded) = PathExpander::expand_tilde("~/test.txt") {
            assert!(expanded.ends_with("test.txt"));
            assert!(!expanded.starts_with("~"));
        }
    }

    #[test]
    fn test_common_prefix() {
        let completion = PathCompletion::new();

        let candidates = vec![
            "test1.txt".to_string(),
            "test2.txt".to_string(),
            "test3.md".to_string(),
        ];

        let prefix = completion.find_common_prefix(&candidates);
        assert_eq!(prefix, "test");

        let single_candidate = vec!["unique.txt".to_string()];
        let prefix = completion.find_common_prefix(&single_candidate);
        assert_eq!(prefix, "unique.txt");

        let empty_candidates: Vec<String> = vec![];
        let prefix = completion.find_common_prefix(&empty_candidates);
        assert_eq!(prefix, "");
    }

    #[test]
    fn test_completion_result_methods() {
        let empty_result = CompletionResult {
            candidates: vec![],
            common_prefix: String::new(),
            is_directory_completion: false,
        };
        assert!(empty_result.is_empty());
        assert!(!empty_result.is_single_match());

        let single_result = CompletionResult {
            candidates: vec!["test.txt".to_string()],
            common_prefix: "test.txt".to_string(),
            is_directory_completion: false,
        };
        assert!(!single_result.is_empty());
        assert!(single_result.is_single_match());
        assert!(single_result.is_exact_match("test.txt"));

        let multiple_result = CompletionResult {
            candidates: vec!["test1.txt".to_string(), "test2.txt".to_string()],
            common_prefix: "test".to_string(),
            is_directory_completion: false,
        };
        assert!(!multiple_result.is_empty());
        assert!(!multiple_result.is_single_match());
        assert!(!multiple_result.is_exact_match("test"));
    }
}
