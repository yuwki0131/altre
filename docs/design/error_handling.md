# エラーハンドリング設計仕様

## 概要

Altre エディタMVPにおける一貫したエラーハンドリング戦略の設計仕様書。ユーザーフレンドリーなエラー表示と開発時のデバッグ効率向上を両立させる。

## 設計方針

### 基本原則
1. **安全性優先**: 致命的エラー時は即座に強制終了（QA Q11）
2. **開発者向け最適化**: 詳細なデバッグ情報を表示（QA Q12）
3. **一貫性**: 全モジュールで統一されたエラー処理
4. **拡張性**: 将来のロギング・ユーザビリティ向上に対応

### エラーレベル分類
1. **Info**: 情報メッセージ（操作完了通知等）
2. **Warning**: 警告（設定不備、非推奨操作等）
3. **Error**: 回復可能エラー（ファイル読み込み失敗等）
4. **Fatal**: 致命的エラー（メモリ不足、システムリソース不足等）

## エラー分類と処理方針

### 1. ファイルI/Oエラー

#### 分類
- **FileNotFound**: ファイルが存在しない
- **PermissionDenied**: アクセス権限がない
- **InvalidPath**: 無効なパス形式
- **ReadError**: ファイル読み込み失敗
- **WriteError**: ファイル書き込み失敗
- **EncodingError**: 文字エンコーディングエラー

#### 処理方針
```rust
// ユーザーフレンドリーなメッセージでミニバッファに表示（5秒間）
// 操作は継続可能、ファイル操作のみキャンセル
match error {
    FileNotFound(path) => show_error(format!("ファイルが見つかりません: {}", path)),
    PermissionDenied(path) => show_error(format!("アクセス権限がありません: {}", path)),
    // ...
}
```

### 2. UTF-8エンコーディングエラー

#### 分類
- **InvalidUtf8Sequence**: 不正なUTF-8バイト列
- **InvalidCharBoundary**: 文字境界エラー
- **UnicodeNormalizationError**: Unicode正規化エラー

#### 処理方針
```rust
// バイト境界を調整して可能な限り復旧
// 復旧不可能な場合はユーザーに通知し、操作をキャンセル
match utf8_error {
    InvalidUtf8Sequence => {
        // 有効な文字境界まで巻き戻し
        fallback_to_valid_boundary();
        show_warning("一部の文字が正しく表示できませんでした");
    }
}
```

### 3. システムリソースエラー

#### 分類
- **OutOfMemory**: メモリ不足
- **FileSystemFull**: ディスク容量不足
- **TooManyOpenFiles**: ファイルディスクリプタ不足
- **SystemCallFailure**: システムコール失敗

#### 処理方針
```rust
// 致命的エラー：即座に強制終了（QA Q11の回答）
match system_error {
    OutOfMemory => {
        eprintln!("FATAL: Memory exhausted");
        std::process::exit(1);
    }
    FileSystemFull => {
        eprintln!("FATAL: Disk full");
        std::process::exit(1);
    }
}
```

### 4. ユーザー入力エラー

#### 分類
- **InvalidCommand**: 存在しないコマンド
- **InvalidKeySequence**: 無効なキーシーケンス
- **InvalidArgument**: 不正な引数
- **ParseError**: パースエラー

#### 処理方針
```rust
// サイレント無視または軽微な通知（QA Q6の回答）
match input_error {
    InvalidKeySequence => {
        // サイレント無視
    }
    InvalidCommand(cmd) => {
        show_error(format!("コマンドが見つかりません: {}", cmd));
    }
}
```

### 5. 内部ロジックエラー

#### 分類
- **AssertionFailure**: アサート失敗
- **InvalidState**: 不正な内部状態
- **ConcurrencyError**: 並行処理エラー
- **ConfigurationError**: 設定エラー

#### 処理方針
```rust
// 開発者向け詳細情報を表示して即座に終了
match logic_error {
    AssertionFailure(msg) => {
        eprintln!("ASSERTION FAILED: {} at {}:{}", msg, file!(), line!());
        std::process::exit(1);
    }
}
```

## エラーデータ構造設計

### カスタムエラー型階層

```rust
use thiserror::Error;

/// アプリケーション全体のエラー型
#[derive(Error, Debug)]
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
}

/// ファイル操作固有のエラー
#[derive(Error, Debug)]
pub enum FileError {
    #[error("File not found: {path}")]
    NotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("Encoding error: {message}")]
    Encoding { message: String },

    #[error("IO error: {source}")]
    Io { #[from] source: std::io::Error },
}

/// バッファ操作固有のエラー
#[derive(Error, Debug)]
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
#[derive(Error, Debug)]
pub enum UiError {
    #[error("Terminal initialization failed")]
    TerminalInit,

    #[error("Screen size too small: {width}x{height}")]
    ScreenTooSmall { width: u16, height: u16 },

    #[error("Rendering failed: {component}")]
    RenderingFailed { component: String },
}

/// 入力処理固有のエラー
#[derive(Error, Debug)]
pub enum InputError {
    #[error("Invalid key sequence: {sequence}")]
    InvalidKeySequence { sequence: String },

    #[error("Command not found: {command}")]
    CommandNotFound { command: String },

    #[error("Invalid argument: {arg}")]
    InvalidArgument { arg: String },
}

/// システム固有のエラー
#[derive(Error, Debug)]
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
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration file: {path}")]
    InvalidFile { path: String },

    #[error("Missing required setting: {key}")]
    MissingRequired { key: String },

    #[error("Invalid value for {key}: {value}")]
    InvalidValue { key: String, value: String },
}
```

### エラーコンテキスト管理

```rust
use anyhow::{Context, Result};

/// エラーにコンテキスト情報を追加するためのトレイト
pub trait ErrorContext<T> {
    fn with_context_info(self, operation: &str, location: &str) -> Result<T>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context_info(self, operation: &str, location: &str) -> Result<T> {
        self.with_context(|| format!("Operation '{}' failed at {}", operation, location))
    }
}

/// ファイル操作のコンテキスト付きエラーハンドリング例
pub fn read_file_with_context(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context_info("file_read", &format!("{}:{}", file!(), line!()))
        .context(format!("Failed to read file: {}", path))
}
```

## エラー表示戦略

### ミニバッファでのエラー表示

```rust
pub struct ErrorDisplay {
    /// エラーメッセージ
    pub message: String,
    /// エラーレベル
    pub level: ErrorLevel,
    /// 表示開始時刻
    pub start_time: std::time::Instant,
    /// 表示持続時間（QA Q10: 5秒）
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorLevel {
    Info,
    Warning,
    Error,
    Fatal,
}

impl ErrorDisplay {
    pub fn new(error: &AltreError) -> Self {
        let (message, level) = Self::format_error(error);
        Self {
            message,
            level,
            start_time: std::time::Instant::now(),
            duration: std::time::Duration::from_secs(5), // QA Q10の回答
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
            AltreError::System(SystemError::OutOfMemory) => {
                ("メモリ不足のため終了します".to_string(), ErrorLevel::Fatal)
            }
            // ... 他のエラーパターン
            _ => {
                (format!("エラーが発生しました: {}", error), ErrorLevel::Error)
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }
}
```

### 日本語エラーメッセージ辞書

```rust
use std::collections::HashMap;

pub struct ErrorMessageDictionary {
    messages: HashMap<String, String>,
}

impl ErrorMessageDictionary {
    pub fn new() -> Self {
        let mut messages = HashMap::new();

        // ファイル操作関連
        messages.insert("file_not_found".to_string(), "ファイルが見つかりません".to_string());
        messages.insert("permission_denied".to_string(), "アクセス権限がありません".to_string());
        messages.insert("invalid_path".to_string(), "無効なパスです".to_string());
        messages.insert("read_error".to_string(), "ファイル読み込みに失敗しました".to_string());
        messages.insert("write_error".to_string(), "ファイル書き込みに失敗しました".to_string());

        // バッファ操作関連
        messages.insert("buffer_overflow".to_string(), "バッファ容量を超過しました".to_string());
        messages.insert("invalid_cursor".to_string(), "無効なカーソル位置です".to_string());
        messages.insert("utf8_error".to_string(), "文字エンコーディングエラーです".to_string());

        // システム関連
        messages.insert("out_of_memory".to_string(), "メモリ不足です".to_string());
        messages.insert("disk_full".to_string(), "ディスク容量が不足しています".to_string());

        // UI関連
        messages.insert("terminal_init_failed".to_string(), "ターミナル初期化に失敗しました".to_string());
        messages.insert("screen_too_small".to_string(), "画面サイズが小さすぎます".to_string());

        Self { messages }
    }

    pub fn get_message(&self, key: &str) -> Option<&String> {
        self.messages.get(key)
    }

    pub fn get_message_or_default(&self, key: &str, default: &str) -> &str {
        self.messages.get(key).map(|s| s.as_str()).unwrap_or(default)
    }
}
```

## ログ出力設計

### ログレベルと出力先

```rust
use log::{debug, info, warn, error};

pub enum LogLevel {
    Debug,   // 詳細なデバッグ情報
    Info,    // 一般的な情報
    Warning, // 警告
    Error,   // エラー
    Fatal,   // 致命的エラー
}

pub struct Logger {
    level: LogLevel,
    output_stderr: bool,  // 開発者向け：stderrに出力
    output_file: Option<String>,  // 将来的なファイル出力
}

impl Logger {
    /// 開発者向けロガー（QA Q12の回答）
    pub fn for_development() -> Self {
        Self {
            level: LogLevel::Debug,
            output_stderr: true,
            output_file: None,
        }
    }

    pub fn log_error(&self, error: &AltreError, context: Option<&str>) {
        let context_info = context.unwrap_or("unknown");

        match error {
            AltreError::System(_) => {
                error!("FATAL ERROR in {}: {:?}", context_info, error);
                // 詳細なスタックトレースを出力（QA Q12の回答）
                error!("Stack trace: {}", std::backtrace::Backtrace::capture());
            }
            _ => {
                error!("ERROR in {}: {:?}", context_info, error);
                debug!("Error details: {:#?}", error);
            }
        }
    }

    pub fn log_warning(&self, message: &str, context: Option<&str>) {
        let context_info = context.unwrap_or("unknown");
        warn!("WARNING in {}: {}", context_info, message);
    }

    pub fn log_info(&self, message: &str, context: Option<&str>) {
        let context_info = context.unwrap_or("unknown");
        info!("INFO in {}: {}", context_info, message);
    }

    pub fn log_debug(&self, message: &str, file: &str, line: u32) {
        debug!("DEBUG at {}:{}: {}", file, line, message);
    }
}

// 便利マクロ
macro_rules! log_debug_here {
    ($logger:expr, $msg:expr) => {
        $logger.log_debug($msg, file!(), line!())
    };
}
```

### エラーレポート生成

```rust
pub struct ErrorReport {
    pub error: AltreError,
    pub timestamp: std::time::SystemTime,
    pub context: String,
    pub stack_trace: String,
    pub system_info: SystemInfo,
}

#[derive(Debug)]
pub struct SystemInfo {
    pub os: String,
    pub architecture: String,
    pub memory_usage: u64,
    pub version: String,
}

impl ErrorReport {
    pub fn generate(error: AltreError, context: &str) -> Self {
        Self {
            error,
            timestamp: std::time::SystemTime::now(),
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

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            memory_usage: Self::get_memory_usage(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn get_memory_usage() -> u64 {
        // プロセスメモリ使用量を取得（プラットフォーム依存）
        // 実装は簡略化
        0
    }
}
```

## Result型の一貫した使用

### プロジェクト全体のResult型

```rust
/// プロジェクト標準のResult型
pub type Result<T> = std::result::Result<T, AltreError>;

/// 各モジュール固有のResult型（必要に応じて）
pub mod file {
    pub type Result<T> = std::result::Result<T, super::FileError>;
}

pub mod buffer {
    pub type Result<T> = std::result::Result<T, super::BufferError>;
}
```

### エラー変換の自動化

```rust
// From トレイトを活用した自動変換
impl From<std::io::Error> for AltreError {
    fn from(error: std::io::Error) -> Self {
        AltreError::File(FileError::Io { source: error })
    }
}

impl From<std::str::Utf8Error> for AltreError {
    fn from(error: std::str::Utf8Error) -> Self {
        AltreError::Buffer(BufferError::Utf8Boundary {
            position: error.valid_up_to()
        })
    }
}

// ? 演算子での簡潔なエラーハンドリング
pub fn example_function() -> Result<String> {
    let content = std::fs::read_to_string("example.txt")?; // 自動変換
    let processed = process_content(&content)?;
    Ok(processed)
}
```

## パニック回避戦略

### パニック防止ガイドライン

```rust
// ❌ パニックを起こす可能性があるコード
fn bad_example(vec: &Vec<String>, index: usize) -> &String {
    &vec[index]  // index が範囲外の場合パニック
}

// ✅ Result型を使った安全なコード
fn good_example(vec: &Vec<String>, index: usize) -> Result<&String> {
    vec.get(index)
        .ok_or(AltreError::Buffer(BufferError::InvalidCursorPosition { position: index }))
}

// ✅ Option型との組み合わせ
fn safe_string_access(s: &str, start: usize, end: usize) -> Result<&str> {
    if start <= end && end <= s.len() && s.is_char_boundary(start) && s.is_char_boundary(end) {
        Ok(&s[start..end])
    } else {
        Err(AltreError::Buffer(BufferError::Utf8Boundary { position: start }))
    }
}
```

### パニックハンドラの設定

```rust
use std::panic;

pub fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().unwrap_or_else(|| {
            panic::Location::caller()
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
```

## 統合例：ファイル操作でのエラーハンドリング

```rust
use crate::error::*;

pub struct FileManager {
    logger: Logger,
    error_display: Option<ErrorDisplay>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            logger: Logger::for_development(),
            error_display: None,
        }
    }

    pub fn open_file(&mut self, path: &str) -> Result<String> {
        self.logger.log_info(&format!("Opening file: {}", path), Some("file_manager"));

        let result = self.try_open_file(path);

        match &result {
            Ok(_) => {
                self.logger.log_info(&format!("Successfully opened: {}", path), Some("file_manager"));
            }
            Err(error) => {
                self.logger.log_error(error, Some("file_manager"));
                self.error_display = Some(ErrorDisplay::new(error));
            }
        }

        result
    }

    fn try_open_file(&self, path: &str) -> Result<String> {
        // パス検証
        if path.is_empty() {
            return Err(AltreError::File(FileError::InvalidPath {
                path: path.to_string()
            }));
        }

        // ファイル読み込み
        let content = std::fs::read_to_string(path)
            .map_err(|io_error| {
                match io_error.kind() {
                    std::io::ErrorKind::NotFound => {
                        AltreError::File(FileError::NotFound { path: path.to_string() })
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        AltreError::File(FileError::PermissionDenied { path: path.to_string() })
                    }
                    _ => {
                        AltreError::File(FileError::Io { source: io_error })
                    }
                }
            })?;

        // UTF-8検証
        if !content.is_ascii() && !content.chars().all(|c| c.is_ascii() || c as u32 <= 0x10FFFF) {
            return Err(AltreError::File(FileError::Encoding {
                message: "Invalid UTF-8 content".to_string()
            }));
        }

        Ok(content)
    }

    pub fn get_current_error_display(&self) -> Option<&ErrorDisplay> {
        self.error_display.as_ref().filter(|display| !display.is_expired())
    }

    pub fn clear_expired_errors(&mut self) {
        if let Some(display) = &self.error_display {
            if display.is_expired() {
                self.error_display = None;
            }
        }
    }
}
```

## テスト戦略

### エラーハンドリングのテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_not_found_error() {
        let mut file_manager = FileManager::new();
        let result = file_manager.open_file("/nonexistent/file.txt");

        assert!(result.is_err());
        match result.unwrap_err() {
            AltreError::File(FileError::NotFound { path }) => {
                assert_eq!(path, "/nonexistent/file.txt");
            }
            _ => panic!("Expected FileNotFound error"),
        }

        // エラー表示の確認
        let error_display = file_manager.get_current_error_display();
        assert!(error_display.is_some());
        assert_eq!(error_display.unwrap().level, ErrorLevel::Error);
    }

    #[test]
    fn test_error_message_expiry() {
        let error = AltreError::File(FileError::NotFound {
            path: "test.txt".to_string()
        });
        let mut display = ErrorDisplay::new(&error);

        assert!(!display.is_expired());

        // 時間経過をシミュレート
        display.start_time = std::time::Instant::now() - std::time::Duration::from_secs(6);
        assert!(display.is_expired());
    }

    #[test]
    fn test_panic_prevention() {
        let vec = vec!["a".to_string(), "b".to_string()];

        // 安全なアクセス
        let result = good_example(&vec, 5);
        assert!(result.is_err());

        match result.unwrap_err() {
            AltreError::Buffer(BufferError::InvalidCursorPosition { position }) => {
                assert_eq!(position, 5);
            }
            _ => panic!("Expected InvalidCursorPosition error"),
        }
    }
}
```

## 将来の拡張計画

### フェーズ2: ユーザビリティ向上
- エラーメッセージの多言語対応
- インタラクティブなエラー復旧オプション
- 設定可能なエラー表示時間
- グラフィカルなエラー表示

### フェーズ3: 高度なロギング
- 構造化ログ（JSON形式）
- ログローテーション
- リモートログ送信
- パフォーマンス監視

### フェーズ4: 統合監視
- エラー発生傾向の分析
- 自動バグレポート生成
- ユーザーフィードバックシステム
- A/Bテスト用エラー表示バリエーション

この設計により、MVP段階での堅牢なエラーハンドリングと将来の拡張性を両立させる。