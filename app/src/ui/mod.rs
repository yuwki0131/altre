//! UIモジュール
//!
//! ratatuiベースのターミナルUI機能

pub mod layout;
pub mod text_area;
pub mod minibuffer;
pub mod viewport;
pub mod theme;
pub mod renderer;

// 公開API
pub use layout::{LayoutManager, AppLayout, AreaType};
pub use text_area::{TextArea, TextAreaRenderer};
pub use minibuffer::MinibufferRenderer;
pub use viewport::ViewportManager;
pub use theme::{ThemeManager, Theme, ComponentType, ThemeType};
pub use renderer::{AdvancedRenderer, FrameRateStats, RenderStats};
