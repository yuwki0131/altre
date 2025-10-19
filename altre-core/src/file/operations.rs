//! ファイル操作コア機能
//!
//! ファイルオープン、保存、バッファ管理の実装

use crate::error::{AltreError, FileError, Result};
use crate::file::metadata::{EncodingProcessor, FileChangeTracker, FileInfo, LineEndingProcessor};
use std::path::{Path, PathBuf};

/// ファイル操作用デバッグマクロ
macro_rules! file_debug_log {
    ($self:expr, $($arg:tt)*) => {
        if $self.debug_mode {
            eprintln!("DEBUG FileSaver: {}", format!($($arg)*));
        }
    };
}

/// ファイル読み込み処理
pub struct FileReader;

impl FileReader {
    pub fn new() -> Self {
        Self
    }

    /// ファイル内容を読み込み
    pub fn read_file(&self, path: &Path) -> Result<String> {
        let file_info = FileInfo::analyze(path)?;

        // 存在チェック
        if !file_info.exists {
            return Ok(String::new()); // 新規ファイル
        }

        // ファイル種別チェック
        if !file_info.is_file {
            return Err(AltreError::File(FileError::InvalidPath {
                path: format!("Not a regular file: {}", path.display()),
            }));
        }

        // 権限チェック（QA Q19: エラー表示）
        if !file_info.is_readable {
            return Err(AltreError::File(FileError::PermissionDenied {
                path: path.display().to_string(),
            }));
        }

        // ファイル読み込み
        let content = std::fs::read_to_string(path).map_err(|e| {
            AltreError::File(FileError::Io {
                message: e.to_string(),
            })
        })?;

        // BOM除去
        let without_bom = EncodingProcessor::remove_bom(&content);

        // 改行コード統一
        let normalized_content = LineEndingProcessor::normalize_to_lf(without_bom);

        Ok(normalized_content)
    }

    /// ファイル内容の検証
    pub fn validate_content(&self, content: &str) -> Result<()> {
        // 制御文字チェック（タブと改行以外）
        for (pos, ch) in content.char_indices() {
            if ch.is_control() && ch != '\t' && ch != '\n' {
                log::warn!("Control character found at position {}: {:?}", pos, ch);
            }
        }

        Ok(())
    }
}

impl Default for FileReader {
    fn default() -> Self {
        Self
    }
}

/// ファイル保存処理
pub struct FileSaver {
    atomic_save: bool,
    debug_mode: bool,
}

impl FileSaver {
    pub fn new() -> Self {
        Self {
            atomic_save: true,
            debug_mode: std::env::var("ALTRE_DEBUG").is_ok(),
        }
    }

    /// ファイルを保存
    pub fn save_file(&self, path: &Path, content: &str) -> Result<()> {
        // バックアップなし（QA Q16の回答）

        file_debug_log!(self, "save_file called with path: {}", path.display());
        file_debug_log!(self, "content length: {}", content.len());

        // LF改行コード統一
        let save_content = LineEndingProcessor::ensure_lf_endings(content);
        file_debug_log!(self, "normalized content length: {}", save_content.len());

        // アトミック保存実装
        let result = if self.atomic_save {
            file_debug_log!(self, "using atomic save");
            self.atomic_save_impl(path, &save_content)
        } else {
            file_debug_log!(self, "using direct save");
            self.direct_save_impl(path, &save_content)
        };

        match &result {
            Ok(_) => file_debug_log!(self, "save operation completed successfully"),
            Err(e) => file_debug_log!(self, "save operation failed: {}", e),
        }

        result
    }

    /// アトミック保存（一時ファイル経由）
    fn atomic_save_impl(&self, path: &Path, content: &str) -> Result<()> {
        let temp_path = self.generate_temp_path(path)?;
        file_debug_log!(self, "atomic_save: temp_path: {}", temp_path.display());

        // 一時ファイルに書き込み
        file_debug_log!(self, "atomic_save: writing to temp file");
        std::fs::write(&temp_path, content.as_bytes()).map_err(|e| {
            file_debug_log!(self, "atomic_save: write to temp failed: {}", e);
            AltreError::File(FileError::Io {
                message: e.to_string(),
            })
        })?;

        file_debug_log!(self, "atomic_save: temp file written successfully");

        // 原子的にリネーム
        file_debug_log!(
            self,
            "atomic_save: renaming {} to {}",
            temp_path.display(),
            path.display()
        );
        std::fs::rename(&temp_path, path).map_err(|e| {
            file_debug_log!(self, "atomic_save: rename failed: {}", e);
            // 一時ファイル削除を試行
            let _ = std::fs::remove_file(&temp_path);
            AltreError::File(FileError::Io {
                message: e.to_string(),
            })
        })?;

        file_debug_log!(self, "atomic_save: rename completed successfully");
        Ok(())
    }

    /// 直接保存
    fn direct_save_impl(&self, path: &Path, content: &str) -> Result<()> {
        std::fs::write(path, content.as_bytes()).map_err(|e| {
            AltreError::File(FileError::Io {
                message: e.to_string(),
            })
        })
    }

    fn generate_temp_path(&self, original: &Path) -> Result<PathBuf> {
        let parent = original.parent().ok_or_else(|| {
            AltreError::File(FileError::InvalidPath {
                path: original.display().to_string(),
            })
        })?;

        let filename = original.file_name().ok_or_else(|| {
            AltreError::File(FileError::InvalidPath {
                path: original.display().to_string(),
            })
        })?;

        // 一意な一時ファイル名生成
        let temp_name = format!(".{}_{}", filename.to_string_lossy(), std::process::id());

        Ok(parent.join(temp_name))
    }

    /// 内容の事前検証
    pub fn validate_save_content(&self, content: &str) -> Result<()> {
        // 改行コード統一確認
        if content.contains("\r\n") || content.contains('\r') {
            log::warn!("Non-LF line endings detected, will be normalized");
        }

        Ok(())
    }
}

impl Default for FileSaver {
    fn default() -> Self {
        Self::new()
    }
}

/// ファイルバッファ管理
#[derive(Clone)]
pub struct FileBuffer {
    /// ファイルパス
    pub path: Option<PathBuf>,
    /// バッファ名
    pub name: String,
    /// テキスト内容
    pub content: String,
    /// 変更追跡
    pub change_tracker: FileChangeTracker,
    /// ファイル情報
    pub file_info: Option<FileInfo>,
    /// 読み取り専用フラグ
    pub read_only: bool,
}

impl FileBuffer {
    /// ファイルから新しいバッファを作成
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let file_info = FileInfo::analyze(&path)?;

        let content = if file_info.exists {
            FileReader::new().read_file(&path)?
        } else {
            String::new()
        };

        Ok(FileBuffer {
            name: Self::generate_buffer_name(&path),
            path: Some(path),
            content: content.clone(),
            change_tracker: FileChangeTracker::new(&content),
            file_info: Some(file_info),
            read_only: false,
        })
    }

    /// 新規バッファを作成
    pub fn new_empty(name: String) -> Self {
        FileBuffer {
            name,
            path: None,
            content: String::new(),
            change_tracker: FileChangeTracker::new(""),
            file_info: None,
            read_only: false,
        }
    }

    /// バッファ名生成
    fn generate_buffer_name(path: &Path) -> String {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    /// 変更状態チェック
    pub fn is_modified(&self) -> bool {
        self.change_tracker.is_modified(&self.content)
    }

    /// 保存処理
    pub fn save(&mut self) -> Result<()> {
        let path = self.path.as_ref().ok_or_else(|| {
            AltreError::File(FileError::InvalidPath {
                path: "No file associated with buffer".to_string(),
            })
        })?;

        // 保存実行
        FileSaver::new().save_file(path, &self.content)?;

        // 変更状態リセット
        self.change_tracker.mark_saved(&self.content);

        Ok(())
    }

    /// 別名で保存
    pub fn save_as(&mut self, path: PathBuf) -> Result<()> {
        NewFileHandler::handle_new_file(&path)?;
        self.set_path(path.clone());
        FileSaver::new().save_file(&path, &self.content)?;
        self.change_tracker.mark_saved(&self.content);
        self.refresh_file_info()?;
        Ok(())
    }

    /// ファイル情報更新
    pub fn refresh_file_info(&mut self) -> Result<()> {
        if let Some(path) = &self.path {
            self.file_info = Some(FileInfo::analyze(path)?);
        }
        Ok(())
    }

    /// パスを設定（新規ファイル保存時）
    pub fn set_path(&mut self, path: PathBuf) {
        self.name = Self::generate_buffer_name(&path);
        self.path = Some(path);
    }
}

/// 新規ファイル処理
pub struct NewFileHandler;

impl NewFileHandler {
    pub fn handle_new_file(path: &Path) -> Result<String> {
        // 親ディレクトリの存在確認（より柔軟に）
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                // 親ディレクトリが存在しない場合は作成を試みる
                if let Err(_) = std::fs::create_dir_all(parent) {
                    return Err(AltreError::File(FileError::InvalidPath {
                        path: format!("Cannot create directory: {}", parent.display()),
                    }));
                }
            }

            // 簡略化した書き込み権限確認
            if !parent.exists() || !parent.is_dir() {
                return Err(AltreError::File(FileError::PermissionDenied {
                    path: parent.display().to_string(),
                }));
            }
        }

        // 新規ファイルは空文字列で開始
        Ok(String::new())
    }
}

/// ファイル操作統合マネージャー
pub struct FileOperationManager;

impl FileOperationManager {
    pub fn new() -> Self {
        Self
    }

    /// ファイルを開く
    pub fn open_file(&mut self, path: PathBuf) -> Result<FileBuffer> {
        FileBuffer::from_file(path)
    }

    /// バッファを保存
    pub fn save_buffer(&mut self, buffer: &mut FileBuffer) -> Result<()> {
        // 変更チェック
        if !buffer.is_modified() {
            return Ok(()); // 変更なし
        }

        buffer.save()
    }

    /// バッファを別名で保存
    pub fn save_buffer_as(&mut self, buffer: &mut FileBuffer, path: PathBuf) -> Result<()> {
        buffer.save_as(path)
    }

    /// ファイル存在チェック
    ///
    /// # Examples
    /// ```
    /// use altre::file::operations::FileOperationManager;
    ///
    /// let mut manager = FileOperationManager::new();
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("sample.txt");
    /// std::fs::write(&path, "hello").unwrap();
    /// assert!(manager.file_exists(&path));
    /// std::fs::remove_file(&path).unwrap();
    /// assert!(!manager.file_exists(&path));
    /// ```
    pub fn file_exists(&self, path: &Path) -> bool {
        path.exists() && path.is_file()
    }

    /// 新規ファイル用バッファ作成
    pub fn create_new_file_buffer(&self, path: PathBuf) -> Result<FileBuffer> {
        // 新規ファイル処理
        let content = NewFileHandler::handle_new_file(&path)?;

        Ok(FileBuffer {
            name: FileBuffer::generate_buffer_name(&path),
            path: Some(path),
            content,
            change_tracker: FileChangeTracker::new(""),
            file_info: None,
            read_only: false,
        })
    }
}

impl Default for FileOperationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_reader_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("nonexistent.txt");

        let reader = FileReader::new();
        let content = reader.read_file(&test_file).unwrap();

        assert_eq!(content, "");
    }

    #[test]
    fn test_file_reader_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        fs::write(&test_file, "hello\r\nworld\rtest").unwrap();

        let reader = FileReader::new();
        let content = reader.read_file(&test_file).unwrap();

        // 改行コードがLFに統一されている
        assert_eq!(content, "hello\nworld\ntest");
    }

    #[test]
    fn test_file_saver_no_backup() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let saver = FileSaver::new();
        saver.save_file(&test_file, "test content").unwrap();

        // ファイルが保存されている
        assert!(test_file.exists());
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "test content");

        // バックアップファイルが作成されていない（QA Q16）
        assert!(!temp_dir.path().join("test.txt~").exists());
        assert!(!temp_dir.path().join("test.txt.bak").exists());
    }

    #[test]
    fn test_file_buffer_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // 新規ファイルバッファ作成
        let mut buffer = FileBuffer::from_file(test_file.clone()).unwrap();
        assert_eq!(buffer.content, "");
        assert!(!buffer.is_modified());

        // 内容変更
        buffer.content = "modified content".to_string();
        assert!(buffer.is_modified());

        // 保存
        buffer.save().unwrap();
        assert!(!buffer.is_modified());

        // ファイルが作成されている
        assert!(test_file.exists());
        let saved_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(saved_content, "modified content");
    }

    #[test]
    fn test_line_ending_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // CRLF改行でファイル作成
        fs::write(&test_file, "line1\r\nline2\r\nline3").unwrap();

        let reader = FileReader::new();
        let content = reader.read_file(&test_file).unwrap();

        // LFに統一されている
        assert_eq!(content, "line1\nline2\nline3");

        // 保存時もLFが維持される
        let saver = FileSaver::new();
        saver.save_file(&test_file, &content).unwrap();

        let saved_content = fs::read(&test_file).unwrap();
        assert_eq!(saved_content, b"line1\nline2\nline3");
    }

    #[test]
    fn test_symlink_handling() {
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("target.txt");
        let link_file = temp_dir.path().join("link.txt");

        // ターゲットファイル作成
        fs::write(&target_file, "target content").unwrap();

        // シンボリックリンク作成
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_file, &link_file).unwrap();

            // QA Q18: シンボリックリンクは基本対応（リンク先ファイル直接編集）
            let file_info = FileInfo::analyze(&link_file).unwrap();
            assert!(file_info.is_symlink);
            assert_eq!(file_info.path, target_file);

            let reader = FileReader::new();
            let content = reader.read_file(&link_file).unwrap();
            assert_eq!(content, "target content");
        }
    }
}
