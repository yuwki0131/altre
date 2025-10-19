use std::{fmt, io};

use crate::alisp::symbol::SymbolId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceSpan {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

impl SourceSpan {
    pub fn single_point(line: usize, column: usize) -> Self {
        let loc = SourceLocation::new(line, column);
        Self {
            start: loc.clone(),
            end: loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReaderErrorKind {
    UnexpectedChar(char),
    UnterminatedString,
    InvalidNumber(String),
    UnexpectedToken(String),
    UnexpectedEof,
    Io(io::ErrorKind),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReaderError {
    pub kind: ReaderErrorKind,
    pub span: SourceSpan,
    pub message: String,
}

impl ReaderError {
    pub fn new(kind: ReaderErrorKind, span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for ReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "reader error at {}:{} - {}",
            self.span.start.line, self.span.start.column, self.message
        )
    }
}

impl std::error::Error for ReaderError {}

#[derive(Debug, Clone, PartialEq)]
pub enum EvalErrorKind {
    NameNotFound(SymbolId),
    ArityMismatch {
        expected: usize,
        found: usize,
    },
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    DivisionByZero,
    InvalidLetBinding,
    InvalidDefineTarget,
    Reader(ReaderError),
    Runtime(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalError {
    pub kind: EvalErrorKind,
    pub span: Option<SourceSpan>,
    pub message: String,
}

impl EvalError {
    pub fn new(kind: EvalErrorKind, span: Option<SourceSpan>, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
        }
    }

    pub fn from_reader(err: ReaderError) -> Self {
        Self::new(
            EvalErrorKind::Reader(err.clone()),
            Some(err.span.clone()),
            err.message,
        )
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.span {
            Some(span) => write!(
                f,
                "{}:{}: {}",
                span.start.line, span.start.column, self.message
            ),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for EvalError {}
