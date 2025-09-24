//! ファイルメタデータ管理
//!
//! ファイル情報の追跡と管理システム

use crate::error::{AltreError, FileError, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::io::ErrorKind;

/// ファイル情報
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub exists: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_readable: bool,
    pub is_writable: bool,
    pub size: u64,
    pub modified: SystemTime,
}

impl FileInfo {
    /// ファイル情報を分析
    pub fn analyze(path: &Path) -> Result<Self> {
        let metadata = match path.symlink_metadata() {
            Ok(meta) => meta,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // 新規ファイル
                return Ok(FileInfo {
                    path: path.to_path_buf(),
                    exists: false,
                    is_file: false,
                    is_symlink: false,
                    is_readable: false,
                    is_writable: Self::can_create_file(path)?,
                    size: 0,
                    modified: SystemTime::UNIX_EPOCH,
                });
            }
            Err(e) => return Err(AltreError::File(FileError::Io { message: e.to_string() })),
        };

        // シンボリックリンク処理（QA Q18: 基本対応）
        let (actual_path, is_symlink) = if metadata.is_symlink() {
            match path.canonicalize() {
                Ok(target) => (target, true),
                Err(_) => return Err(AltreError::File(FileError::InvalidPath {
                    path: format!("Broken symlink: {}", path.display())
                })),
            }
        } else {
            (path.to_path_buf(), false)
        };

        // 実際のファイルメタデータ取得
        let file_metadata = actual_path.metadata()
            .map_err(|e| AltreError::File(FileError::Io { message: e.to_string() }))?;

        Ok(FileInfo {
            path: actual_path.clone(),
            exists: true,
            is_file: file_metadata.is_file(),
            is_symlink,
            is_readable: Self::test_readable(&actual_path),
            is_writable: Self::test_writable(&actual_path),
            size: file_metadata.len(),
            modified: file_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        })
    }

    /// 読み取り権限テスト
    pub fn test_readable(path: &Path) -> bool {
        std::fs::File::open(path).is_ok()
    }

    /// 書き込み権限テスト
    pub fn test_writable(path: &Path) -> bool {
        if path.is_file() {
            // ファイルが存在する場合は書き込み権限をテスト
            std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(path)
                .is_ok()
        } else if path.is_dir() {
            // ディレクトリの場合は一時ファイル作成でテスト
            let temp_file = path.join(".tmp_write_test");
            let result = std::fs::write(&temp_file, "").is_ok();
            if result {
                let _ = std::fs::remove_file(&temp_file);
            }
            result
        } else {
            // ファイルもディレクトリも存在しない場合は親ディレクトリをチェック
            if let Some(parent) = path.parent() {
                if parent.exists() && parent.is_dir() {
                    Self::test_writable(parent)
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    /// 新規ファイル作成可能性チェック
    fn can_create_file(path: &Path) -> Result<bool> {
        if let Some(parent) = path.parent() {
            if parent.exists() {
                Ok(Self::test_writable(parent))
            } else {
                Err(AltreError::File(FileError::InvalidPath {
                    path: format!("Parent directory does not exist: {}", parent.display())
                }))
            }
        } else {
            Ok(false)
        }
    }
}

/// ファイル変更追跡
#[derive(Debug, Clone)]
pub struct FileChangeTracker {
    original_content: String,
    original_hash: u64,
    last_saved: SystemTime,
}

impl FileChangeTracker {
    pub fn new(content: &str) -> Self {
        Self {
            original_content: content.to_string(),
            original_hash: Self::calculate_hash(content),
            last_saved: SystemTime::now(),
        }
    }

    /// 変更状態チェック
    pub fn is_modified(&self, current_content: &str) -> bool {
        Self::calculate_hash(current_content) != self.original_hash
    }

    /// 保存状態マーク
    pub fn mark_saved(&mut self, content: &str) {
        self.original_content = content.to_string();
        self.original_hash = Self::calculate_hash(content);
        self.last_saved = SystemTime::now();
    }

    /// 内容のハッシュ計算
    fn calculate_hash(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// 最後の保存時刻
    pub fn last_saved_time(&self) -> SystemTime {
        self.last_saved
    }

    /// 元の内容
    pub fn original_content(&self) -> &str {
        &self.original_content
    }
}

/// 改行コードスタイル
#[derive(Debug, PartialEq, Clone)]
pub enum LineEndingStyle {
    Lf,     // \n (Unix)
    Crlf,   // \r\n (Windows)
    Cr,     // \r (Classic Mac)
    Mixed,  // 混在
    None,   // 改行なし
}

/// 改行コード処理
pub struct LineEndingProcessor;

impl LineEndingProcessor {
    /// 改行コードをLFに統一
    pub fn normalize_to_lf(content: &str) -> String {
        // CRLF (\r\n) を LF (\n) に変換
        let step1 = content.replace("\r\n", "\n");

        // 残りの CR (\r) を LF (\n) に変換
        step1.replace('\r', "\n")
    }

    /// 改行コード検出
    pub fn detect_line_endings(content: &str) -> LineEndingStyle {
        let has_crlf = content.contains("\r\n");
        let has_lf = content.contains('\n');
        let has_cr = content.contains('\r');

        match (has_crlf, has_lf, has_cr) {
            (true, _, _) => LineEndingStyle::Crlf,
            (false, true, false) => LineEndingStyle::Lf,
            (false, false, true) => LineEndingStyle::Cr,
            (false, true, true) => LineEndingStyle::Mixed,
            (false, false, false) => LineEndingStyle::None,
        }
    }

    /// 保存時のLF確認
    pub fn ensure_lf_endings(content: &str) -> String {
        // 常にLFに統一
        Self::normalize_to_lf(content)
    }
}

/// エンコーディング処理
pub struct EncodingProcessor;

impl EncodingProcessor {
    /// BOM除去処理
    pub fn remove_bom(content: &str) -> &str {
        // UTF-8 BOM (EF BB BF) を除去
        content.strip_prefix('\u{FEFF}').unwrap_or(content)
    }

    /// ファイル読み込み時のUTF-8検証・変換
    pub fn process_file_content(raw_content: &[u8]) -> Result<String> {
        // UTF-8として解釈を試行
        match std::str::from_utf8(raw_content) {
            Ok(content) => Ok(Self::remove_bom(content).to_string()),
            Err(utf8_error) => {
                // UTF-8でない場合はエラー
                Err(AltreError::File(FileError::Encoding {
                    message: format!("ファイルはUTF-8エンコーディングである必要があります: {}", utf8_error)
                }))
            }
        }
    }

    /// 保存時のUTF-8変換
    pub fn prepare_save_content(content: &str) -> Vec<u8> {
        // 既にUTF-8文字列なのでそのままバイト列に変換
        content.as_bytes().to_vec()
    }
}

/// ファイルメタデータ管理
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub permissions: std::fs::Permissions,
    pub is_symlink: bool,
    pub encoding: String,
    pub line_ending: LineEndingStyle,
}

impl FileMetadata {
    pub fn from_file(path: &Path) -> Result<Self> {
        let metadata = path.metadata()
            .map_err(|e| AltreError::File(FileError::Io { message: e.to_string() }))?;

        let symlink_metadata = path.symlink_metadata()
            .map_err(|e| AltreError::File(FileError::Io { message: e.to_string() }))?;

        Ok(FileMetadata {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            permissions: metadata.permissions(),
            is_symlink: symlink_metadata.is_symlink(),
            encoding: "UTF-8".to_string(), // 強制UTF-8
            line_ending: LineEndingStyle::Lf, // 強制LF
        })
    }

    /// 外部変更検出
    pub fn has_changed_externally(&self) -> Result<bool> {
        if !self.path.exists() {
            return Ok(true); // ファイルが削除された
        }

        let current_metadata = FileMetadata::from_file(&self.path)?;
        Ok(current_metadata.modified != self.modified ||
           current_metadata.size != self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_line_ending_normalization() {
        assert_eq!(
            LineEndingProcessor::normalize_to_lf("hello\r\nworld\r\ntest"),
            "hello\nworld\ntest"
        );

        assert_eq!(
            LineEndingProcessor::normalize_to_lf("hello\rworld\rtest"),
            "hello\nworld\ntest"
        );

        assert_eq!(
            LineEndingProcessor::normalize_to_lf("hello\nworld\ntest"),
            "hello\nworld\ntest"
        );
    }

    #[test]
    fn test_line_ending_detection() {
        assert_eq!(
            LineEndingProcessor::detect_line_endings("hello\r\nworld"),
            LineEndingStyle::Crlf
        );

        assert_eq!(
            LineEndingProcessor::detect_line_endings("hello\nworld"),
            LineEndingStyle::Lf
        );

        assert_eq!(
            LineEndingProcessor::detect_line_endings("hello\rworld"),
            LineEndingStyle::Cr
        );

        assert_eq!(
            LineEndingProcessor::detect_line_endings("hello world"),
            LineEndingStyle::None
        );
    }

    #[test]
    fn test_bom_removal() {
        let content_with_bom = "\u{FEFF}hello world";
        assert_eq!(
            EncodingProcessor::remove_bom(content_with_bom),
            "hello world"
        );

        let content_without_bom = "hello world";
        assert_eq!(
            EncodingProcessor::remove_bom(content_without_bom),
            "hello world"
        );
    }

    #[test]
    fn test_file_info_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // 新規ファイル
        let file_info = FileInfo::analyze(&test_file).unwrap();
        assert!(!file_info.exists);
        assert!(!file_info.is_file);

        // ファイル作成後
        fs::write(&test_file, "test content").unwrap();
        let file_info = FileInfo::analyze(&test_file).unwrap();
        assert!(file_info.exists);
        assert!(file_info.is_file);
        assert!(file_info.is_readable);
        assert!(file_info.is_writable);
    }

    #[test]
    fn test_change_tracker() {
        let mut tracker = FileChangeTracker::new("original content");

        // 初期状態は変更なし
        assert!(!tracker.is_modified("original content"));

        // 内容変更
        assert!(tracker.is_modified("modified content"));

        // 保存後は変更なし
        tracker.mark_saved("modified content");
        assert!(!tracker.is_modified("modified content"));
    }

    #[test]
    fn test_symlink_handling() {
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("target.txt");
        let link_file = temp_dir.path().join("link.txt");

        // ターゲットファイル作成
        fs::write(&target_file, "target content").unwrap();

        // シンボリックリンク作成（Unix系のみ）
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_file, &link_file).unwrap();

            let file_info = FileInfo::analyze(&link_file).unwrap();
            assert!(file_info.is_symlink);
            assert_eq!(file_info.path, target_file); // リンク先パスが取得される
        }
    }
}