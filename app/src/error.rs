//! エラーハンドリングシステム
//!
//! Altre エディタ全体で使用される統一されたエラー型とユーティリティを定義
//! QA回答に基づく設計：致命的エラーは即座に強制終了、開発者向け詳細表示

use crate::logging::{LogLevel, Logger};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;

/// アプリケーション全体のエラー型
#[derive(Error, Debug, Clone)]
pub enum AltreError {
    /// ファイル操作エラー
    #[error("File operation failed")]
    File(#[from] FileError),

    /// バッファ操作エラー
    #[error("Buffer operation failed")]
    Buffer(#[from] BufferError),

    /// UI操作エラー
    #[error("UI operation failed")]
    Ui(#[from] UiError),

    /// 入力処理エラー
    #[error("Input processing failed")]
    Input(#[from] InputError),

    /// システムエラー
    #[error("System error")]
    System(#[from] SystemError),

    /// 設定エラー
    #[error("Configuration error")]
    Config(#[from] ConfigError),

    /// アプリケーション論理エラー
    #[error("Application error: {0}")]
    Application(String),

    /// パスエラー
    #[error("Path error: {0}")]
    Path(String),

    /// 編集操作エラー
    #[error("Edit error: {0}")]
    Edit(String),
}

/// ファイル操作固有のエラー
#[derive(Error, Debug, Clone)]
pub enum FileError {
    #[error("File not found: {path}")]
    NotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("Encoding error: {message}")]
    Encoding { message: String },

    #[error("IO error: {message}")]
    Io { message: String },
}

/// バッファ操作固有のエラー
#[derive(Error, Debug, Clone)]
pub enum BufferError {
    #[error("Invalid cursor position: {position}")]
    InvalidCursorPosition { position: usize },

    #[error("Buffer overflow")]
    Overflow,

    #[error("UTF-8 boundary error at position {position}")]
    Utf8Boundary { position: usize },

    #[error("Empty buffer")]
    Empty,
}

/// UI操作固有のエラー
#[derive(Error, Debug, Clone)]
pub enum UiError {
    #[error("Terminal initialization failed")]
    TerminalInit,

    #[error("Screen size too small: {width}x{height}")]
    ScreenTooSmall { width: u16, height: u16 },

    #[error("Rendering failed: {component}")]
    RenderingFailed { component: String },
}

/// 入力処理固有のエラー
#[derive(Error, Debug, Clone)]
pub enum InputError {
    #[error("Invalid key sequence: {sequence}")]
    InvalidKeySequence { sequence: String },

    #[error("Command not found: {command}")]
    CommandNotFound { command: String },

    #[error("Invalid argument: {arg}")]
    InvalidArgument { arg: String },
}

/// システム固有のエラー（致命的）
#[derive(Error, Debug, Clone)]
pub enum SystemError {
    #[error("Out of memory")]
    OutOfMemory,

    #[error("File system full")]
    FileSystemFull,

    #[error("Too many open files")]
    TooManyOpenFiles,

    #[error("System call failed: {call}")]
    SystemCallFailed { call: String },
}

/// 設定固有のエラー
#[derive(Error, Debug, Clone)]
pub enum ConfigError {
    #[error("Invalid configuration file: {path}")]
    InvalidFile { path: String },

    #[error("Missing required setting: {key}")]
    MissingRequired { key: String },

    #[error("Invalid value for {key}: {value}")]
    InvalidValue { key: String, value: String },
}

/// エラーレベル分類
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorLevel {
    Info,
    Warning,
    Error,
    Fatal,
}

/// 日本語エラーメッセージ辞書
static ERROR_MESSAGE_CATALOG: OnceLock<ErrorMessageCatalog> = OnceLock::new();

struct MessageEntry {
    text: &'static str,
    level: ErrorLevel,
}

struct ErrorMessageCatalog {
    entries: HashMap<&'static str, MessageEntry>,
}

impl ErrorMessageCatalog {
    fn new() -> Self {
        use ErrorLevel::*;

        let mut entries = HashMap::new();

        entries.insert(
            "file_not_found",
            MessageEntry {
                text: "ファイルが見つかりません",
                level: Error,
            },
        );
        entries.insert(
            "permission_denied",
            MessageEntry {
                text: "アクセス権限がありません",
                level: Error,
            },
        );
        entries.insert(
            "invalid_path",
            MessageEntry {
                text: "無効なパスです",
                level: Error,
            },
        );
        entries.insert(
            "encoding_error",
            MessageEntry {
                text: "文字エンコーディングエラー",
                level: Error,
            },
        );
        entries.insert(
            "io_error",
            MessageEntry {
                text: "ファイル操作中にエラーが発生しました",
                level: Error,
            },
        );
        entries.insert(
            "buffer_invalid_cursor",
            MessageEntry {
                text: "無効なカーソル位置です",
                level: Error,
            },
        );
        entries.insert(
            "buffer_utf8",
            MessageEntry {
                text: "文字境界エラーが発生しました",
                level: Warning,
            },
        );
        entries.insert(
            "buffer_overflow",
            MessageEntry {
                text: "バッファ容量を超過しました",
                level: Error,
            },
        );
        entries.insert(
            "system_out_of_memory",
            MessageEntry {
                text: "メモリ不足のため終了します",
                level: Fatal,
            },
        );
        entries.insert(
            "system_disk_full",
            MessageEntry {
                text: "ディスク容量不足のため終了します",
                level: Fatal,
            },
        );
        entries.insert(
            "system_too_many_files",
            MessageEntry {
                text: "開いているファイルが多すぎます",
                level: Fatal,
            },
        );
        entries.insert(
            "system_call_failed",
            MessageEntry {
                text: "システムコールが失敗しました",
                level: Error,
            },
        );
        entries.insert(
            "input_invalid_command",
            MessageEntry {
                text: "コマンドが見つかりません",
                level: Error,
            },
        );
        entries.insert(
            "input_invalid_key_sequence",
            MessageEntry {
                text: "無効なキーシーケンスです",
                level: Warning,
            },
        );
        entries.insert(
            "input_invalid_argument",
            MessageEntry {
                text: "無効な引数です",
                level: Error,
            },
        );
        entries.insert(
            "ui_terminal_init",
            MessageEntry {
                text: "ターミナル初期化に失敗しました",
                level: Fatal,
            },
        );
        entries.insert(
            "ui_screen_too_small",
            MessageEntry {
                text: "画面サイズが小さすぎます",
                level: Error,
            },
        );
        entries.insert(
            "ui_rendering_failed",
            MessageEntry {
                text: "描画に失敗しました",
                level: Error,
            },
        );
        entries.insert(
            "config_invalid_file",
            MessageEntry {
                text: "無効な設定ファイルです",
                level: Error,
            },
        );
        entries.insert(
            "config_missing_required",
            MessageEntry {
                text: "必須設定が不足しています",
                level: Error,
            },
        );
        entries.insert(
            "config_invalid_value",
            MessageEntry {
                text: "設定値が無効です",
                level: Error,
            },
        );
        entries.insert(
            "application",
            MessageEntry {
                text: "アプリケーションエラーが発生しました",
                level: Error,
            },
        );
        entries.insert(
            "path",
            MessageEntry {
                text: "パスエラーが発生しました",
                level: Error,
            },
        );
        entries.insert(
            "edit",
            MessageEntry {
                text: "編集操作でエラーが発生しました",
                level: Error,
            },
        );
        entries.insert(
            "generic_error",
            MessageEntry {
                text: "エラーが発生しました",
                level: Error,
            },
        );

        Self { entries }
    }

    fn compose(&self, key: &str, detail: Option<String>) -> (String, ErrorLevel) {
        if let Some(entry) = self.entries.get(key) {
            let message = match detail {
                Some(detail) if !detail.is_empty() => {
                    format!("{}: {}", entry.text, detail)
                }
                _ => entry.text.to_string(),
            };
            (message, entry.level.clone())
        } else {
            (
                detail.unwrap_or_else(|| "不明なエラーが発生しました".to_string()),
                ErrorLevel::Error,
            )
        }
    }
}

fn message_catalog() -> &'static ErrorMessageCatalog {
    ERROR_MESSAGE_CATALOG.get_or_init(ErrorMessageCatalog::new)
}

/// エラー表示情報
#[derive(Debug, Clone)]
pub struct ErrorDisplay {
    /// エラーメッセージ
    pub message: String,
    /// エラーレベル
    pub level: ErrorLevel,
    /// 表示開始時刻
    pub start_time: Instant,
    /// 表示持続時間（QA Q10: 5秒）
    pub duration: Duration,
}

impl ErrorDisplay {
    pub fn new(error: &AltreError) -> Self {
        let (message, level) = Self::format_error(error);
        Self {
            message,
            level,
            start_time: Instant::now(),
            duration: Duration::from_secs(5), // QA Q10の回答
        }
    }

    fn format_error(error: &AltreError) -> (String, ErrorLevel) {
        let catalog = message_catalog();

        let mapped = match error {
            AltreError::File(FileError::NotFound { path }) => {
                Some(catalog.compose("file_not_found", Some(path.clone())))
            }
            AltreError::File(FileError::PermissionDenied { path }) => {
                Some(catalog.compose("permission_denied", Some(path.clone())))
            }
            AltreError::File(FileError::InvalidPath { path }) => {
                Some(catalog.compose("invalid_path", Some(path.clone())))
            }
            AltreError::File(FileError::Encoding { message }) => {
                Some(catalog.compose("encoding_error", Some(message.clone())))
            }
            AltreError::File(FileError::Io { message }) => {
                Some(catalog.compose("io_error", Some(message.clone())))
            }
            AltreError::Buffer(BufferError::InvalidCursorPosition { position }) => {
                Some(catalog.compose("buffer_invalid_cursor", Some(position.to_string())))
            }
            AltreError::Buffer(BufferError::Utf8Boundary { position }) => {
                Some(catalog.compose("buffer_utf8", Some(format!("位置 {}", position))))
            }
            AltreError::Buffer(BufferError::Overflow) => {
                Some(catalog.compose("buffer_overflow", None))
            }
            AltreError::Buffer(BufferError::Empty) => {
                Some(catalog.compose("buffer_invalid_cursor", Some("バッファが空です".to_string())))
            }
            AltreError::System(SystemError::OutOfMemory) => {
                Some(catalog.compose("system_out_of_memory", None))
            }
            AltreError::System(SystemError::FileSystemFull) => {
                Some(catalog.compose("system_disk_full", None))
            }
            AltreError::System(SystemError::TooManyOpenFiles) => {
                Some(catalog.compose("system_too_many_files", None))
            }
            AltreError::System(SystemError::SystemCallFailed { call }) => {
                Some(catalog.compose("system_call_failed", Some(call.clone())))
            }
            AltreError::Input(InputError::CommandNotFound { command }) => {
                Some(catalog.compose("input_invalid_command", Some(command.clone())))
            }
            AltreError::Input(InputError::InvalidKeySequence { sequence }) => {
                Some(catalog.compose("input_invalid_key_sequence", Some(sequence.clone())))
            }
            AltreError::Input(InputError::InvalidArgument { arg }) => {
                Some(catalog.compose("input_invalid_argument", Some(arg.clone())))
            }
            AltreError::Ui(UiError::TerminalInit) => {
                Some(catalog.compose("ui_terminal_init", None))
            }
            AltreError::Ui(UiError::ScreenTooSmall { width, height }) => {
                Some(catalog.compose("ui_screen_too_small", Some(format!("{}x{}", width, height))))
            }
            AltreError::Ui(UiError::RenderingFailed { component }) => {
                Some(catalog.compose("ui_rendering_failed", Some(component.clone())))
            }
            AltreError::Config(ConfigError::InvalidFile { path }) => {
                Some(catalog.compose("config_invalid_file", Some(path.clone())))
            }
            AltreError::Config(ConfigError::MissingRequired { key }) => {
                Some(catalog.compose("config_missing_required", Some(key.clone())))
            }
            AltreError::Config(ConfigError::InvalidValue { key, value }) => {
                Some(catalog.compose("config_invalid_value", Some(format!("{} = {}", key, value))))
            }
            AltreError::Application(message) => {
                Some(catalog.compose("application", Some(message.clone())))
            }
            AltreError::Path(message) => {
                Some(catalog.compose("path", Some(message.clone())))
            }
            AltreError::Edit(message) => {
                Some(catalog.compose("edit", Some(message.clone())))
            }
        };

        mapped.unwrap_or_else(|| catalog.compose("generic_error", Some(error.to_string())))
    }

    pub fn is_expired(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }
}

fn log_error_internal(logger: &Logger, error: &AltreError, context: Option<&str>) {
    let severity = if matches!(error, AltreError::System(_)) {
        LogLevel::Fatal
    } else {
        LogLevel::Error
    };

    let tag = match severity {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warning => "WARNING",
        LogLevel::Error => "ERROR",
        LogLevel::Fatal => "FATAL ERROR",
    };

    let context_info = context.unwrap_or("unknown");
    logger.log(
        severity,
        format!("{} in {}: {:?}", tag, context_info, error),
    );

    if matches!(severity, LogLevel::Fatal) {
        logger.log(
            LogLevel::Fatal,
            format!("Stack trace: {}", std::backtrace::Backtrace::capture()),
        );
    }
}

/// 致命的エラー処理（QA Q11の回答）
pub fn handle_fatal_error(error: &AltreError, context: &str) -> ! {
    let logger = Logger::for_development();
    log_error_internal(&logger, error, Some(context));

    logger.log(LogLevel::Fatal, "FATAL: Application will terminate immediately");
    std::process::exit(1);
}

/// パニックハンドラの設定
pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().unwrap_or_else(|| {
            std::panic::Location::caller()
        });

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s
        } else {
            "Unknown panic payload"
        };

        let logger = Logger::for_development();
        let context = format!("{}:{}", location.file(), location.line());
        logger.log_fatal_with_trace(
            format!("PANIC: {}", message),
            Some(&context),
        );

        // 致命的エラーとして即座に終了（QA Q11の回答）
        std::process::exit(1);
    }));
}

/// エラーコンテキスト付与のためのトレイト
pub trait ErrorContext<T> {
    fn with_context_info(self, operation: &str, location: &str) -> Result<T>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<AltreError>,
{
    fn with_context_info(self, operation: &str, location: &str) -> Result<T> {
        self.map_err(|e| {
            let error = e.into();
            // ログ出力
            let logger = Logger::for_development();
            log_error_internal(
                &logger,
                &error,
                Some(&format!("{}:{}", operation, location)),
            );
            error
        })
    }
}

/// プロジェクト標準のResult型
pub type Result<T> = std::result::Result<T, AltreError>;

/// 各モジュール固有のResult型
pub mod file {
    pub type Result<T> = std::result::Result<T, super::FileError>;
}

pub mod buffer {
    pub type Result<T> = std::result::Result<T, super::BufferError>;
}

/// エラーレポート
#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub error: AltreError,
    pub timestamp: SystemTime,
    pub context: String,
    pub stack_trace: String,
    pub system_info: SystemInfo,
}

impl ErrorReport {
    pub fn generate(error: AltreError, context: &str) -> Self {
        Self {
            error,
            timestamp: SystemTime::now(),
            context: context.to_string(),
            stack_trace: format!("{}", std::backtrace::Backtrace::capture()),
            system_info: SystemInfo::collect(),
        }
    }

    pub fn format_for_developer(&self) -> String {
        format!(
            "=== Altre Editor Error Report ===\n\
             Time: {:?}\n\
             Context: {}\n\
             Error: {:?}\n\
             Stack Trace:\n{}\n\
             System Info:\n{:?}\n\
             ================================",
            self.timestamp,
            self.context,
            self.error,
            self.stack_trace,
            self.system_info
        )
    }
}

/// システム情報
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub architecture: String,
    pub memory_usage: u64,
    pub version: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            memory_usage: Self::current_memory_usage(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn current_memory_usage() -> u64 {
        // TODO: プラットフォーム別に詳細実装（MVPでは0を返す）
        0
    }
}

/// 編集操作エラー型
#[derive(Error, Debug)]
pub enum EditError {
    #[error("Position {0} is out of bounds")]
    OutOfBounds(usize),

    #[error("Position {0} is not on a character boundary")]
    NotCharBoundary(usize),

    #[error("Invalid character: {0:?}")]
    InvalidChar(char),

    #[error("Cursor at buffer start")]
    AtBufferStart,

    #[error("Cursor at buffer end")]
    AtBufferEnd,

    #[error("Buffer operation failed: {0}")]
    BufferError(String),

    #[error("Memory allocation failed")]
    OutOfMemory,

    #[error("Operation cancelled")]
    Cancelled,
}

// EditError から AltreError への変換
impl From<EditError> for AltreError {
    fn from(error: EditError) -> Self {
        AltreError::Edit(error.to_string())
    }
}

// BufferError から EditError への変換
impl From<BufferError> for EditError {
    fn from(error: BufferError) -> Self {
        match error {
            BufferError::InvalidCursorPosition { position } => EditError::OutOfBounds(position),
            BufferError::Utf8Boundary { position } => EditError::NotCharBoundary(position),
            BufferError::Overflow => EditError::OutOfMemory,
            BufferError::Empty => EditError::AtBufferStart,
        }
    }
}

// std::io::Error から AltreError への変換
impl From<std::io::Error> for AltreError {
    fn from(error: std::io::Error) -> Self {
        AltreError::File(FileError::Io { message: error.to_string() })
    }
}

// UTF-8エラーの変換
impl From<std::str::Utf8Error> for AltreError {
    fn from(error: std::str::Utf8Error) -> Self {
        AltreError::Buffer(BufferError::Utf8Boundary {
            position: error.valid_up_to()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_creation() {
        let error = AltreError::File(FileError::NotFound {
            path: "test.txt".to_string()
        });
        let display = ErrorDisplay::new(&error);

        assert_eq!(display.level, ErrorLevel::Error);
        assert!(display.message.contains("ファイルが見つかりません"));
        assert!(!display.is_expired());
    }

    #[test]
    fn test_error_display_expiry() {
        let error = AltreError::File(FileError::NotFound {
            path: "test.txt".to_string()
        });
        let mut display = ErrorDisplay::new(&error);

        assert!(!display.is_expired());

        // 時間経過をシミュレート
        display.start_time = Instant::now() - Duration::from_secs(6);
        assert!(display.is_expired());
    }

    #[test]
    fn test_fatal_error_detection() {
        let fatal_error = AltreError::System(SystemError::OutOfMemory);
        let display = ErrorDisplay::new(&fatal_error);

        assert_eq!(display.level, ErrorLevel::Fatal);
        assert!(display.message.contains("メモリ不足"));
    }

    #[test]
    fn test_utf8_error_conversion() {
        let utf8_error = std::str::from_utf8(&[0xff, 0xfe]).unwrap_err();
        let altre_error: AltreError = utf8_error.into();

        match altre_error {
            AltreError::Buffer(BufferError::Utf8Boundary { position }) => {
                assert_eq!(position, 0);
            }
            _ => panic!("Expected Utf8Boundary error"),
        }
    }

    #[test]
    fn test_error_message_catalog_permission_denied() {
        let error = AltreError::File(FileError::PermissionDenied {
            path: "/tmp/test".to_string()
        });
        let display = ErrorDisplay::new(&error);

        assert_eq!(display.level, ErrorLevel::Error);
        assert!(display.message.contains("アクセス権限がありません"));
        assert!(display.message.contains("/tmp/test"));
    }

    #[test]
    fn test_error_report_generation() {
        let error = AltreError::Application("テストエラー".to_string());
        let report = ErrorReport::generate(error, "unit_test");
        let formatted = report.format_for_developer();

        assert!(formatted.contains("Altre Editor Error Report"));
        assert!(formatted.contains("unit_test"));
        assert!(formatted.contains("テストエラー"));
    }
}
