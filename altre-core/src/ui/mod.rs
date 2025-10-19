//! UIモジュール
//!
//! ratatuiベースのターミナルUI機能

pub mod layout;
pub mod minibuffer;
pub mod renderer;
pub mod text_area;
pub mod theme;
pub mod viewport;
pub mod window_manager;

// 公開API
pub use layout::{AppLayout, AreaType, LayoutManager};
pub use minibuffer::MinibufferRenderer;
pub use renderer::{AdvancedRenderer, FrameRateStats, RenderStats, StatusLineInfo};
pub use text_area::{TextArea, TextAreaRenderer};
pub use theme::{ComponentType, Theme, ThemeManager, ThemeType};
pub use viewport::{ViewportManager, ViewportState};
pub use window_manager::{SplitOrientation, WindowError, WindowId, WindowManager};
