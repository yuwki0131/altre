//! UIモジュール
//!
//! ratatuiベースのターミナルUI機能

pub mod layout;
pub mod text_area;
pub mod minibuffer;

// 公開API
pub use layout::{LayoutManager, AppLayout};
pub use text_area::TextArea;
pub use minibuffer::MinibufferRenderer;
