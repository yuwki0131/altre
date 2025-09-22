//! パス処理ユーティリティ
//!
//! ファイルパスの正規化、展開、検証機能

use crate::error::{AltreError, Result};
use std::path::{Path, PathBuf, Component};
use std::env;

/// パス処理のトレイト
pub trait PathProcessor {
    /// パスを正規化（. や .. を解決）
    fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf>;

    /// ホームディレクトリを展開（~ → /home/user）
    fn expand_home<P: AsRef<Path>>(path: P) -> Result<PathBuf>;

    /// 環境変数を展開（$VAR → 値）
    fn expand_env<P: AsRef<Path>>(path: P) -> Result<PathBuf>;

    /// 相対パスを絶対パスに変換
    fn to_absolute<P: AsRef<Path>>(path: P) -> Result<PathBuf>;

    /// パスが安全かどうかチェック（パストラバーサル対策）
    fn is_safe_path<P: AsRef<Path>>(path: P) -> bool;
}

/// パス処理の実装
pub struct DefaultPathProcessor;

impl PathProcessor for DefaultPathProcessor {
    fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                Component::CurDir => {
                    // . は無視
                    continue;
                }
                Component::ParentDir => {
                    // .. は一つ前のコンポーネントを削除
                    if components.is_empty() {
                        return Err(AltreError::Path(
                            "パスが不正です: ルートを超えた親ディレクトリ参照".to_string()
                        ));
                    }
                    components.pop();
                }
                _ => {
                    components.push(component);
                }
            }
        }

        let mut result = PathBuf::new();
        for component in components {
            result.push(component);
        }

        Ok(result)
    }

    fn expand_home<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        if path_str.starts_with('~') {
            let home_dir = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .map_err(|_| AltreError::Path(
                    "ホームディレクトリが取得できません".to_string()
                ))?;

            let expanded = if path_str == "~" {
                home_dir
            } else if path_str.starts_with("~/") {
                format!("{}{}", home_dir, &path_str[1..])
            } else {
                // ~user形式は未サポート
                return Err(AltreError::Path(
                    "~user形式のパス展開は未サポートです".to_string()
                ));
            };

            Ok(PathBuf::from(expanded))
        } else {
            Ok(path.to_path_buf())
        }
    }

    fn expand_env<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        match shellexpand::env(&path_str) {
            Ok(expanded) => Ok(PathBuf::from(expanded.as_ref())),
            Err(e) => Err(AltreError::Path(
                format!("環境変数展開エラー: {}", e)
            )),
        }
    }

    fn to_absolute<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();

        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            let current_dir = env::current_dir()
                .map_err(|e| AltreError::Path(
                    format!("現在のディレクトリが取得できません: {}", e)
                ))?;
            Ok(current_dir.join(path))
        }
    }

    fn is_safe_path<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();

        // パストラバーサル攻撃の検出
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    // .. の使用を禁止
                    return false;
                }
                Component::Normal(name) => {
                    let name_str = name.to_string_lossy();
                    // 危険な文字の検出
                    if name_str.contains('\0') || name_str.contains('\r') || name_str.contains('\n') {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    }
}

/// パス展開の便利関数
pub fn expand_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();

    // ホームディレクトリ展開
    let expanded = DefaultPathProcessor::expand_home(path)?;

    // 環境変数展開
    let expanded = DefaultPathProcessor::expand_env(expanded)?;

    // 正規化
    let normalized = DefaultPathProcessor::normalize_path(expanded)?;

    // 絶対パス化
    DefaultPathProcessor::to_absolute(normalized)
}

/// パス正規化の便利関数
pub fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    DefaultPathProcessor::normalize_path(path)
}

/// ファイル拡張子の判定
pub fn has_extension<P: AsRef<Path>>(path: P, extensions: &[&str]) -> bool {
    if let Some(ext) = path.as_ref().extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        extensions.iter().any(|&e| e.to_lowercase() == ext_str)
    } else {
        false
    }
}

/// ファイル名（拡張子なし）を取得
pub fn file_stem<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|s| s.to_string())
}

/// パスの深さを計算
pub fn path_depth<P: AsRef<Path>>(path: P) -> usize {
    path.as_ref().components().count()
}

/// 共通の親ディレクトリを見つける
pub fn common_parent<P: AsRef<Path>>(paths: &[P]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    let first_components: Vec<_> = paths[0].as_ref().components().collect();
    let mut common_length = first_components.len();

    for path in paths.iter().skip(1) {
        let components: Vec<_> = path.as_ref().components().collect();
        let min_length = std::cmp::min(common_length, components.len());

        for i in 0..min_length {
            if first_components[i] != components[i] {
                common_length = i;
                break;
            }
        }

        if common_length == 0 {
            return None;
        }
    }

    let mut result = PathBuf::new();
    for component in &first_components[..common_length] {
        result.push(component);
    }

    Some(result)
}

/// 相対パスを計算
pub fn relative_path<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<PathBuf> {
    let from = DefaultPathProcessor::to_absolute(from)?;
    let to = DefaultPathProcessor::to_absolute(to)?;

    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    // 共通部分を見つける
    let mut common_length = 0;
    for (i, (from_comp, to_comp)) in from_components.iter().zip(to_components.iter()).enumerate() {
        if from_comp == to_comp {
            common_length = i + 1;
        } else {
            break;
        }
    }

    // 相対パスを構築
    let mut result = PathBuf::new();

    // from から共通部分までの .. を追加
    for _ in common_length..from_components.len() {
        result.push("..");
    }

    // 共通部分から to までのパスを追加
    for component in &to_components[common_length..] {
        result.push(component);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_normalize_path() {
        let path = PathBuf::from("./a/../b/./c");
        let normalized = DefaultPathProcessor::normalize_path(path).unwrap();
        assert_eq!(normalized, PathBuf::from("b/c"));
    }

    #[test]
    fn test_expand_home() {
        // テスト環境でのホームディレクトリ設定
        env::set_var("HOME", "/home/testuser");

        let path = PathBuf::from("~/documents/file.txt");
        let expanded = DefaultPathProcessor::expand_home(path).unwrap();
        assert_eq!(expanded, PathBuf::from("/home/testuser/documents/file.txt"));
    }

    #[test]
    fn test_is_safe_path() {
        assert!(DefaultPathProcessor::is_safe_path("safe/path/file.txt"));
        assert!(!DefaultPathProcessor::is_safe_path("../unsafe/path"));
        assert!(!DefaultPathProcessor::is_safe_path("path/with\0null"));
    }

    #[test]
    fn test_has_extension() {
        assert!(has_extension("file.txt", &["txt", "md"]));
        assert!(has_extension("file.TXT", &["txt", "md"]));
        assert!(!has_extension("file.pdf", &["txt", "md"]));
        assert!(!has_extension("file", &["txt", "md"]));
    }

    #[test]
    fn test_file_stem() {
        assert_eq!(file_stem("file.txt"), Some("file".to_string()));
        assert_eq!(file_stem("path/to/file.txt"), Some("file".to_string()));
        assert_eq!(file_stem("file"), Some("file".to_string()));
    }

    #[test]
    fn test_path_depth() {
        assert_eq!(path_depth("a/b/c"), 3);
        assert_eq!(path_depth("file.txt"), 1);
        assert_eq!(path_depth(""), 0); // 空文字列は0個のコンポーネント
    }

    #[test]
    fn test_common_parent() {
        let paths = vec![
            PathBuf::from("/home/user/doc1.txt"),
            PathBuf::from("/home/user/doc2.txt"),
            PathBuf::from("/home/user/subdir/doc3.txt"),
        ];

        let common = common_parent(&paths).unwrap();
        assert_eq!(common, PathBuf::from("/home/user"));
    }

    #[test]
    fn test_expand_path() {
        env::set_var("HOME", "/home/testuser");
        env::set_var("PROJECT", "/workspace/project");

        let path = "~/projects/$PROJECT/src";
        let expanded = expand_path(path).unwrap();

        // パスが正しく展開されることを確認
        assert!(expanded.to_string_lossy().contains("/home/testuser"));
        assert!(expanded.to_string_lossy().contains("/workspace/project"));
    }
}