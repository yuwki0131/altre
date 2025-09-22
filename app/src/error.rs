//! エラーハンドリングシステム
//!
//! Altre エディタ全体で使用される統一されたエラー型とユーティリティを定義
//! QA回答に基づく設計：致命的エラーは即座に強制終了、開発者向け詳細表示

// use std::fmt;  // 将来使用予定
use std::time::{Duration, Instant};
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
        match error {
            AltreError::File(FileError::NotFound { path }) => {
                (format!("ファイルが見つかりません: {}", path), ErrorLevel::Error)
            }
            AltreError::File(FileError::PermissionDenied { path }) => {
                (format!("アクセス権限がありません: {}", path), ErrorLevel::Error)
            }
            AltreError::File(FileError::InvalidPath { path }) => {
                (format!("無効なパスです: {}", path), ErrorLevel::Error)
            }
            AltreError::File(FileError::Encoding { message }) => {
                (format!("文字エンコーディングエラー: {}", message), ErrorLevel::Error)
            }
            AltreError::Buffer(BufferError::InvalidCursorPosition { position }) => {
                (format!("無効なカーソル位置です: {}", position), ErrorLevel::Error)
            }
            AltreError::Buffer(BufferError::Utf8Boundary { position }) => {
                (format!("文字境界エラー: {}", position), ErrorLevel::Warning)
            }
            AltreError::System(SystemError::OutOfMemory) => {
                ("メモリ不足のため終了します".to_string(), ErrorLevel::Fatal)
            }
            AltreError::System(SystemError::FileSystemFull) => {
                ("ディスク容量不足のため終了します".to_string(), ErrorLevel::Fatal)
            }
            AltreError::Input(InputError::CommandNotFound { command }) => {
                (format!("コマンドが見つかりません: {}", command), ErrorLevel::Error)
            }
            AltreError::Ui(UiError::TerminalInit) => {
                ("ターミナル初期化に失敗しました".to_string(), ErrorLevel::Fatal)
            }
            AltreError::Ui(UiError::ScreenTooSmall { width, height }) => {
                (format!("画面サイズが小さすぎます: {}x{}", width, height), ErrorLevel::Error)
            }
            _ => {
                (format!("エラーが発生しました: {}", error), ErrorLevel::Error)
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }
}

/// 開発者向けロガー（QA Q12の回答）
pub struct Logger {
    level: ErrorLevel,
    output_stderr: bool,
}

impl Logger {
    /// 開発者向けロガーを作成
    pub fn for_development() -> Self {
        Self {
            level: ErrorLevel::Info,
            output_stderr: true,
        }
    }

    fn should_log(&self, level: ErrorLevel) -> bool {
        use ErrorLevel::*;

        match self.level {
            Info => true,
            Warning => matches!(level, Warning | Error | Fatal),
            Error => matches!(level, Error | Fatal),
            Fatal => matches!(level, Fatal),
        }
    }

    fn emit(&self, level: ErrorLevel, message: String) {
        if !self.should_log(level.clone()) {
            return;
        }

        if self.output_stderr {
            eprintln!("{}", message);
        } else {
            println!("{}", message);
        }
    }

    pub fn log_error(&self, error: &AltreError, context: Option<&str>) {
        let context_info = context.unwrap_or("unknown");

        let severity = if matches!(error, AltreError::System(_)) {
            ErrorLevel::Fatal
        } else {
            ErrorLevel::Error
        };

        let tag = match severity {
            ErrorLevel::Info => "INFO",
            ErrorLevel::Warning => "WARNING",
            ErrorLevel::Error => "ERROR",
            ErrorLevel::Fatal => "FATAL ERROR",
        };

        self.emit(severity.clone(), format!("{} in {}: {:?}", tag, context_info, error));

        if matches!(severity, ErrorLevel::Fatal) {
            self.emit(
                ErrorLevel::Fatal,
                format!("Stack trace: {}", std::backtrace::Backtrace::capture()),
            );
        }
    }

    pub fn log_warning(&self, message: &str, context: Option<&str>) {
        let context_info = context.unwrap_or("unknown");
        self.emit(
            ErrorLevel::Warning,
            format!("WARNING in {}: {}", context_info, message),
        );
    }

    pub fn log_info(&self, message: &str, context: Option<&str>) {
        let context_info = context.unwrap_or("unknown");
        self.emit(
            ErrorLevel::Info,
            format!("INFO in {}: {}", context_info, message),
        );
    }

    pub fn log_debug(&self, message: &str, file: &str, line: u32) {
        self.emit(
            ErrorLevel::Info,
            format!("DEBUG at {}:{}: {}", file, line, message),
        );
    }
}

/// 致命的エラー処理（QA Q11の回答）
pub fn handle_fatal_error(error: &AltreError, context: &str) -> ! {
    let logger = Logger::for_development();
    logger.log_error(error, Some(context));

    eprintln!("FATAL: Application will terminate immediately");
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

        eprintln!("PANIC at {}:{}: {}", location.file(), location.line(), message);
        eprintln!("Stack trace: {}", std::backtrace::Backtrace::capture());

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
            logger.log_error(&error, Some(&format!("{}:{}", operation, location)));
            error
        })
    }
}


// 便利マクロ
#[macro_export]
macro_rules! log_debug_here {
    ($logger:expr, $msg:expr) => {
        $logger.log_debug($msg, file!(), line!())
    };
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
}
