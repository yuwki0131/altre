//! UIモジュール
//!
//! ratatuiベースのターミナルUI機能

pub mod layout;
pub mod text_area;
pub mod minibuffer;
pub mod viewport;
pub mod theme;
pub mod renderer;
pub mod window_manager;

// 公開API
pub use layout::{LayoutManager, AppLayout, AreaType};
pub use text_area::{TextArea, TextAreaRenderer};
pub use minibuffer::MinibufferRenderer;
pub use viewport::{ViewportManager, ViewportState};
pub use theme::{ThemeManager, Theme, ComponentType, ThemeType};
pub use renderer::{AdvancedRenderer, FrameRateStats, RenderStats, StatusLineInfo};
pub use window_manager::{WindowManager, WindowId, SplitOrientation, WindowError};
