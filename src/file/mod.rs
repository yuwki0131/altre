//! ファイル操作モジュール
//!
//! QA回答に基づくファイル操作システム：
//! - バックアップなし（将来alisp設定可能）
//! - 大きなファイル制限なし（将来調整予定）
//! - シンボリックリンク基本対応（リンク先ファイル直接編集）
//! - 権限不足はエラー表示（エディタ継続）
//! - 同時編集検出不要（MVP非対応）

pub mod completion;
pub mod io;
pub mod metadata;
pub mod operations;
pub mod path;

// 基本公開API（既存互換）
pub use io::{read_file, write_file, FileOperations};
pub use path::{expand_path, normalize_path, PathProcessor};

// 新しい公開API
pub use completion::{CompletionDisplay, CompletionResult, PathCompletion};
pub use metadata::{
    EncodingProcessor, FileChangeTracker, FileInfo, FileMetadata, LineEndingProcessor,
    LineEndingStyle,
};
pub use operations::{FileBuffer, FileOperationManager, FileReader, FileSaver};
