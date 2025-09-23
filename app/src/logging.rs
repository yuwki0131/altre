//! ロギングシステム
//!
//! 開発者向けの詳細ログ出力と将来のログ拡張のための基盤を提供

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// ログレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

impl LogLevel {
    fn tag(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
}

/// ロガー
///
/// * QA Q12: 開発者向け詳細ログをstderrへ出力
/// * 将来的なファイル出力にも対応できるようにフィールドを用意
#[derive(Debug, Clone)]
pub struct Logger {
    level: LogLevel,
    output_stderr: bool,
    output_file: Option<PathBuf>,
}

impl Logger {
    /// デフォルト構築
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            output_stderr: true,
            output_file: None,
        }
    }

    /// 開発者向けロガー（QA Q12の要件）
    pub fn for_development() -> Self {
        Self::new(LogLevel::Debug)
    }

    /// ログレベルを取得
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// ログレベルを変更
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// ファイル出力を設定（将来利用を想定）
    pub fn with_file_output<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.output_file = Some(path.into());
        self
    }

    /// 標準エラー出力を無効化（テスト向け）
    #[cfg(test)]
    pub fn without_stderr(mut self) -> Self {
        self.output_stderr = false;
        self
    }

    fn should_log(&self, level: LogLevel) -> bool {
        level >= self.level
    }

    fn write_line(&self, message: &str) {
        if self.output_stderr {
            eprintln!("{}", message);
        } else {
            println!("{}", message);
        }

        if let Some(path) = &self.output_file {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(file, "{}", message);
            }
        }
    }

    /// 任意のログレベルでメッセージを出力
    pub fn log(&self, level: LogLevel, message: impl AsRef<str>) {
        if self.should_log(level) {
            self.write_line(&format!("{}: {}", level.tag(), message.as_ref()));
        }
    }

    /// コンテキスト付きでログを出力
    pub fn log_with_context(
        &self,
        level: LogLevel,
        context: Option<&str>,
        message: impl AsRef<str>,
    ) {
        let context_info = context.unwrap_or("unknown");
        self.log(level, format!("{} in {}", message.as_ref(), context_info));
    }

    /// デバッグログ（呼び出し元情報付き）
    pub fn log_debug(&self, message: impl AsRef<str>, file: &str, line: u32) {
        self.log(
            LogLevel::Debug,
            format!("{} at {}:{}", message.as_ref(), file, line),
        );
    }

    /// 情報ログ
    pub fn log_info(&self, message: impl AsRef<str>, context: Option<&str>) {
        self.log_with_context(LogLevel::Info, context, message);
    }

    /// 警告ログ
    pub fn log_warning(&self, message: impl AsRef<str>, context: Option<&str>) {
        self.log_with_context(LogLevel::Warning, context, message);
    }

    /// エラーログ
    pub fn log_error_message(&self, message: impl AsRef<str>, context: Option<&str>) {
        self.log_with_context(LogLevel::Error, context, message);
    }

    /// 致命的エラーログ（スタックトレース出力付き）
    pub fn log_fatal_with_trace(&self, message: impl AsRef<str>, context: Option<&str>) {
        self.log_with_context(LogLevel::Fatal, context, message.as_ref());
        self.log(
            LogLevel::Fatal,
            format!("Stack trace: {}", std::backtrace::Backtrace::capture()),
        );
    }
}

#[macro_export]
macro_rules! log_debug_here {
    ($logger:expr, $msg:expr) => {
        $logger.log_debug($msg, file!(), line!())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_respects_log_level() {
        let logger = Logger::for_development().without_stderr();
        assert!(logger.should_log(LogLevel::Debug));
        assert!(logger.should_log(LogLevel::Error));

        let info_logger = Logger::for_development().with_level(LogLevel::Info).without_stderr();
        assert!(!info_logger.should_log(LogLevel::Debug));
        assert!(info_logger.should_log(LogLevel::Warning));
    }
}
