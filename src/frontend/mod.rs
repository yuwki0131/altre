pub mod tui;

#[cfg(feature = "gui")]
pub mod gui;

#[cfg(feature = "gui")]
pub use gui::GuiApplication;
pub use tui::TuiApplication;
