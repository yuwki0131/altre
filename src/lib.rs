//! altre - Modern Emacs-inspired text editor
//!
//! MVPモジュール構成とアーキテクチャの実装

// 拡張言語
pub mod alisp;

// コアモジュール
pub mod core;
pub mod error;
pub mod frontend;
pub mod logging;

// データ層
pub mod buffer;
pub mod file;

// 編集層
pub mod editor;

// ロジック層
pub mod input;
pub mod minibuffer;
pub mod search;

// 表示層
pub mod ui;

// パフォーマンス
pub mod performance;

// 公開API
pub use core::Backend;
pub use error::{AltreError, Result};
pub use frontend::TuiApplication;
#[cfg(feature = "gui")]
pub use frontend::{GuiApplication, GuiRunOptions};
