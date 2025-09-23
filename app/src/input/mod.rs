//! 入力処理モジュール
//!
//! キーバインド、コマンド処理、イベントハンドリングを提供

pub mod keybinding;
pub mod commands;
pub mod event_handler;

// 公開API
pub use keybinding::{Action, DeleteDirection, Key, KeyCode, KeyModifiers, KeyProcessResult, ModernKeyMap};
pub use commands::{Command, CommandProcessor, CommandResult};
pub use event_handler::{InputHandler, EventProcessor};
