//! 入力処理モジュール
//!
//! キーバインド、コマンド処理、イベントハンドリングを提供

pub mod commands;
pub mod event_handler;
pub mod keybinding;

// 公開API
pub use commands::{Command, CommandProcessor, CommandResult};
pub use event_handler::{EventProcessor, InputHandler};
pub use keybinding::{
    Action, DeleteDirection, Key, KeyCode, KeyModifiers, KeyProcessResult, ModernKeyMap,
};
