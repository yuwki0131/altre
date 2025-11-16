//! ファイルI/O操作
//!
//! UTF-8テキストファイルの読み込みと保存機能

use crate::error::{AltreError, FileError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// ファイル操作のトレイト
pub trait FileOperations {
    /// ファイルからテキストを読み込み
    fn read_file<P: AsRef<Path>>(path: P) -> Result<String>;

    /// テキストをファイルに書き込み
    fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()>;

    /// ファイルが存在するかチェック
    fn file_exists<P: AsRef<Path>>(path: P) -> bool;

    /// ディレクトリが存在するかチェック
    fn dir_exists<P: AsRef<Path>>(path: P) -> bool;

    /// 親ディレクトリを作成
    fn create_parent_dirs<P: AsRef<Path>>(path: P) -> Result<()>;
}

/// ファイル操作の実装
pub struct DefaultFileOperations;

impl FileOperations for DefaultFileOperations {
    fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
        let path = path.as_ref();

        // ファイル存在チェック
        if !path.exists() {
            return Err(AltreError::File(FileError::NotFound {
                path: path.display().to_string(),
            }));
        }

        // ディレクトリではないことを確認
        if path.is_dir() {
            return Err(AltreError::File(FileError::InvalidPath {
                path: path.display().to_string(),
            }));
        }

        // ファイル読み込み
        let content = fs::read_to_string(path)?;

        // UTF-8検証（read_to_stringで既に行われるが明示的に）
        if !content.is_empty() && content.as_bytes().iter().any(|&b| b > 127) {
            // 非ASCII文字が含まれている場合、UTF-8として有効かチェック
            std::str::from_utf8(content.as_bytes()).map_err(AltreError::from)?;
        }

        Ok(content)
    }

    fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
        let path = path.as_ref();

        // UTF-8検証
        let _ = content.as_bytes(); // UTF-8として有効であることを確認

        // 親ディレクトリが存在しない場合は作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // 一時ファイルに書き込んでからアトミックに移動
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    fn file_exists<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        path.exists() && path.is_file()
    }

    fn dir_exists<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        path.exists() && path.is_dir()
    }

    fn create_parent_dirs<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        Ok(())
    }
}

/// ファイル読み込みの便利関数
pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
    DefaultFileOperations::read_file(path)
}

/// ファイル書き込みの便利関数
pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    DefaultFileOperations::write_file(path, content)
}

/// ファイル情報
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// ファイルパス
    pub path: PathBuf,
    /// ファイルサイズ（バイト）
    pub size: u64,
    /// 最終更新時刻
    pub modified: Option<std::time::SystemTime>,
    /// 読み取り専用かどうか
    pub readonly: bool,
}

impl FileInfo {
    /// 指定パスのファイル情報を取得
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let metadata = fs::metadata(path)?;

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified().ok(),
            readonly: metadata.permissions().readonly(),
        })
    }
}

/// ファイルのバックアップ作成
pub fn create_backup<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();

    if !DefaultFileOperations::file_exists(path) {
        return Err(AltreError::File(FileError::NotFound {
            path: path.display().to_string(),
        }));
    }

    // バックアップファイル名を生成
    let backup_path = generate_backup_path(path);

    // ファイルをコピー
    fs::copy(path, &backup_path)?;

    Ok(backup_path)
}

/// バックアップファイルパスを生成
fn generate_backup_path(path: &Path) -> PathBuf {
    let mut backup_path = path.to_path_buf();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(extension) = path.extension() {
        let new_extension = format!("{}.bak.{}", extension.to_string_lossy(), timestamp);
        backup_path.set_extension(new_extension);
    } else {
        let new_name = format!(
            "{}.bak.{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            timestamp
        );
        backup_path.set_file_name(new_name);
    }

    backup_path
}

/// ファイルエンコーディングの検出（UTF-8のみ対応）
pub fn detect_encoding<P: AsRef<Path>>(path: P) -> Result<String> {
    let content = fs::read(path)?;

    // BOMチェック
    if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok("UTF-8 BOM".to_string());
    }

    // UTF-8検証
    match std::str::from_utf8(&content) {
        Ok(_) => Ok("UTF-8".to_string()),
        Err(_) => Err(AltreError::Application(
            "ファイルがUTF-8エンコーディングではありません".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_write_and_read_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, World!\nこんにちは！";

        // ファイル書き込み
        assert!(write_file(&file_path, content).is_ok());

        // ファイル読み込み
        let read_content = read_file(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_file_exists() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("exists.txt");

        assert!(!DefaultFileOperations::file_exists(&file_path));

        fs::write(&file_path, "test").unwrap();
        assert!(DefaultFileOperations::file_exists(&file_path));
    }

    #[test]
    fn test_create_parent_dirs() {
        let temp_dir = tempdir().unwrap();
        let nested_path = temp_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("file.txt");

        assert!(DefaultFileOperations::create_parent_dirs(&nested_path).is_ok());
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    fn test_backup_creation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("original.txt");
        let content = "Original content";

        fs::write(&file_path, content).unwrap();

        let backup_path = create_backup(&file_path).unwrap();
        assert!(backup_path.exists());

        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, content);
    }

    #[test]
    fn test_encoding_detection() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("utf8.txt");

        fs::write(&file_path, "UTF-8 content").unwrap();
        let encoding = detect_encoding(&file_path).unwrap();
        assert_eq!(encoding, "UTF-8");
    }
}
