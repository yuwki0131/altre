//! ファイル操作モジュール
//!
//! QA回答に基づくファイル操作システム：
//! - バックアップなし（将来alisp設定可能）
//! - 大きなファイル制限なし（将来調整予定）
//! - シンボリックリンク基本対応（リンク先ファイル直接編集）
//! - 権限不足はエラー表示（エディタ継続）
//! - 同時編集検出不要（MVP非対応）

pub mod io;
pub mod path;
pub mod operations;
pub mod completion;
pub mod metadata;

// 基本公開API（既存互換）
pub use io::{FileOperations, read_file, write_file};
pub use path::{PathProcessor, expand_path, normalize_path};

// 新しい公開API
pub use operations::{FileOperationManager, FileBuffer, FileSaver, FileReader};
pub use completion::{PathCompletion, CompletionResult, CompletionDisplay};
pub use metadata::{FileInfo, FileMetadata, FileChangeTracker, LineEndingStyle, EncodingProcessor, LineEndingProcessor};
