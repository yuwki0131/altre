//! altre - Modern Emacs-inspired text editor
//!
//! MVPモジュール構成とアーキテクチャの実装

// コアモジュール
pub mod error;
pub mod logging;
pub mod app;

// データ層
pub mod buffer;
pub mod file;

// 編集層
pub mod editor;

// ロジック層
pub mod input;
pub mod minibuffer;

// 表示層
pub mod ui;

// 公開API
pub use app::App;
pub use error::{AltreError, Result};
