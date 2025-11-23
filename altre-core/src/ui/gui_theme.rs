//! GUI用のカラーテーマ設定
//!
//! React/Tauri フロントエンドで利用する配色を保持し、alisp から上書きできるようにする。

use serde::{Deserialize, Serialize};

/// GUI テーマで指定可能なキー
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuiThemeKey {
    AppBackground,
    AppForeground,
    FocusRing,
    ActiveLineBackground,
    CursorBackground,
    CursorForeground,
    MinibufferBorder,
    MinibufferPrompt,
    MinibufferInput,
    MinibufferInfo,
    MinibufferError,
    StatuslineBorder,
    StatuslineBackground,
    StatuslineForeground,
}

impl GuiThemeKey {
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "app-background" => Some(Self::AppBackground),
            "app-foreground" => Some(Self::AppForeground),
            "focus-ring" => Some(Self::FocusRing),
            "active-line-background" => Some(Self::ActiveLineBackground),
            "cursor-background" => Some(Self::CursorBackground),
            "cursor-foreground" => Some(Self::CursorForeground),
            "minibuffer-border" => Some(Self::MinibufferBorder),
            "minibuffer-prompt" => Some(Self::MinibufferPrompt),
            "minibuffer-input" => Some(Self::MinibufferInput),
            "minibuffer-info" => Some(Self::MinibufferInfo),
            "minibuffer-error" => Some(Self::MinibufferError),
            "statusline-border" => Some(Self::StatuslineBorder),
            "statusline-background" => Some(Self::StatuslineBackground),
            "statusline-foreground" => Some(Self::StatuslineForeground),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            GuiThemeKey::AppBackground => "app-background",
            GuiThemeKey::AppForeground => "app-foreground",
            GuiThemeKey::FocusRing => "focus-ring",
            GuiThemeKey::ActiveLineBackground => "active-line-background",
            GuiThemeKey::CursorBackground => "cursor-background",
            GuiThemeKey::CursorForeground => "cursor-foreground",
            GuiThemeKey::MinibufferBorder => "minibuffer-border",
            GuiThemeKey::MinibufferPrompt => "minibuffer-prompt",
            GuiThemeKey::MinibufferInput => "minibuffer-input",
            GuiThemeKey::MinibufferInfo => "minibuffer-info",
            GuiThemeKey::MinibufferError => "minibuffer-error",
            GuiThemeKey::StatuslineBorder => "statusline-border",
            GuiThemeKey::StatuslineBackground => "statusline-background",
            GuiThemeKey::StatuslineForeground => "statusline-foreground",
        }
    }
}

/// GUI テーマ設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiThemeConfig {
    pub app_background: String,
    pub app_foreground: String,
    pub focus_ring: String,
    pub active_line_background: String,
    pub cursor_background: String,
    pub cursor_foreground: String,
    pub minibuffer_border: String,
    pub minibuffer_prompt: String,
    pub minibuffer_input: String,
    pub minibuffer_info: String,
    pub minibuffer_error: String,
    pub statusline_border: String,
    pub statusline_background: String,
    pub statusline_foreground: String,
}

impl GuiThemeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_color(&mut self, key: GuiThemeKey, value: &str) -> Result<(), String> {
        let normalized = Self::normalize_color(value)?;
        match key {
            GuiThemeKey::AppBackground => self.app_background = normalized,
            GuiThemeKey::AppForeground => self.app_foreground = normalized,
            GuiThemeKey::FocusRing => self.focus_ring = normalized,
            GuiThemeKey::ActiveLineBackground => self.active_line_background = normalized,
            GuiThemeKey::CursorBackground => self.cursor_background = normalized,
            GuiThemeKey::CursorForeground => self.cursor_foreground = normalized,
            GuiThemeKey::MinibufferBorder => self.minibuffer_border = normalized,
            GuiThemeKey::MinibufferPrompt => self.minibuffer_prompt = normalized,
            GuiThemeKey::MinibufferInput => self.minibuffer_input = normalized,
            GuiThemeKey::MinibufferInfo => self.minibuffer_info = normalized,
            GuiThemeKey::MinibufferError => self.minibuffer_error = normalized,
            GuiThemeKey::StatuslineBorder => self.statusline_border = normalized,
            GuiThemeKey::StatuslineBackground => self.statusline_background = normalized,
            GuiThemeKey::StatuslineForeground => self.statusline_foreground = normalized,
        }
        Ok(())
    }

    pub fn color(&self, key: GuiThemeKey) -> &str {
        match key {
            GuiThemeKey::AppBackground => &self.app_background,
            GuiThemeKey::AppForeground => &self.app_foreground,
            GuiThemeKey::FocusRing => &self.focus_ring,
            GuiThemeKey::ActiveLineBackground => &self.active_line_background,
            GuiThemeKey::CursorBackground => &self.cursor_background,
            GuiThemeKey::CursorForeground => &self.cursor_foreground,
            GuiThemeKey::MinibufferBorder => &self.minibuffer_border,
            GuiThemeKey::MinibufferPrompt => &self.minibuffer_prompt,
            GuiThemeKey::MinibufferInput => &self.minibuffer_input,
            GuiThemeKey::MinibufferInfo => &self.minibuffer_info,
            GuiThemeKey::MinibufferError => &self.minibuffer_error,
            GuiThemeKey::StatuslineBorder => &self.statusline_border,
            GuiThemeKey::StatuslineBackground => &self.statusline_background,
            GuiThemeKey::StatuslineForeground => &self.statusline_foreground,
        }
    }

    fn normalize_color(value: &str) -> Result<String, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("カラーコードが空です".to_string());
        }
        if trimmed.len() > 32 {
            return Err("カラーコードが長すぎます (32 文字以内)".to_string());
        }
        if !trimmed.is_ascii() {
            return Err("カラーコードにASCII以外の文字が含まれています".to_string());
        }
        Ok(trimmed.to_string())
    }
}

impl Default for GuiThemeConfig {
    fn default() -> Self {
        Self {
            app_background: "#1c1c1c".to_string(),
            app_foreground: "#f0f0f0".to_string(),
            focus_ring: "#4466ff33".to_string(),
            active_line_background: "#2a2a40".to_string(),
            cursor_background: "#88aaff".to_string(),
            cursor_foreground: "#111".to_string(),
            minibuffer_border: "#2c2c2c".to_string(),
            minibuffer_prompt: "#9fa8ff".to_string(),
            minibuffer_input: "#ffffff".to_string(),
            minibuffer_info: "#ffb86c".to_string(),
            minibuffer_error: "#ff6b6b".to_string(),
            statusline_border: "#2c2c2c".to_string(),
            statusline_background: "#202025".to_string(),
            statusline_foreground: "#d0d0d0".to_string(),
        }
    }
}
