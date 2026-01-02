use altre::buffer::CursorPosition;
use altre::core::RenderMetadata;
use altre::minibuffer::{MinibufferMode, MinibufferSystem};
use altre::ui::viewport::ViewportState;
use altre::ui::GuiThemeConfig;
use serde::{Deserialize, Serialize};
use altre::search::{HighlightKind, SearchHighlight};
use altre::search::{SearchDirection, SearchStatus, SearchUiState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorSnapshot {
    pub buffer: BufferSnapshot,
    pub minibuffer: MinibufferSnapshot,
    pub status: StatusSnapshot,
    pub viewport: ViewportSnapshot,
    pub theme: GuiThemeSnapshot,
    #[serde(rename = "searchUi")]
    pub search_ui: Option<SearchUISnapshot>,
    pub highlights: Vec<HighlightSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BufferSnapshot {
    pub lines: Vec<String>,
    pub cursor: CursorSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CursorSnapshot {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinibufferSnapshot {
    pub mode: String,
    pub prompt: String,
    pub input: String,
    pub completions: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusSnapshot {
    pub label: String,
    pub is_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewportSnapshot {
    pub top_line: usize,
    pub height: usize,
    pub scroll_x: usize,
    pub width: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuiThemeSnapshot {
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

impl EditorSnapshot {
    pub fn new(
        text: &str,
        cursor: &CursorPosition,
        metadata: &RenderMetadata,
        minibuffer: &MinibufferSystem,
        viewport: ViewportState,
        gui_theme: GuiThemeConfig,
    ) -> Self {
        Self {
            buffer: BufferSnapshot::from_text(text, cursor),
            minibuffer: MinibufferSnapshot::from_system(minibuffer),
            status: StatusSnapshot {
                label: metadata.status_label.clone(),
                is_modified: metadata.is_modified,
            },
            viewport: ViewportSnapshot::from(viewport),
            theme: GuiThemeSnapshot::from(gui_theme),
            search_ui: metadata.search_ui.as_ref().map(SearchUISnapshot::from),
            highlights: metadata
                .highlights
                .iter()
                .map(HighlightSnapshot::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HighlightSnapshot {
    pub line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub is_current: bool,
    pub kind: String,
}

impl From<&SearchHighlight> for HighlightSnapshot {
    fn from(h: &SearchHighlight) -> Self {
        Self {
            line: h.line,
            start_column: h.start_column,
            end_column: h.end_column,
            is_current: h.is_current,
            kind: match h.kind {
                HighlightKind::Search => "search".to_string(),
                HighlightKind::Selection => "selection".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchUISnapshot {
    pub prompt_label: String,
    pub pattern: String,
    pub status: String,
    pub current_match: Option<usize>,
    pub total_matches: usize,
    pub wrapped: bool,
    pub message: Option<String>,
    pub direction: String,
}

impl From<&SearchUiState> for SearchUISnapshot {
    fn from(s: &SearchUiState) -> Self {
        Self {
            prompt_label: s.prompt_label.clone(),
            pattern: s.pattern.clone(),
            status: match s.status {
                SearchStatus::Active => "active".to_string(),
                SearchStatus::NotFound => "not-found".to_string(),
                SearchStatus::Wrapped => "wrapped".to_string(),
            },
            current_match: s.current_match,
            total_matches: s.total_matches,
            wrapped: s.wrapped,
            message: s.message.clone(),
            direction: match s.direction {
                SearchDirection::Forward => "forward".to_string(),
                SearchDirection::Backward => "backward".to_string(),
            },
        }
    }
}

impl BufferSnapshot {
    pub fn from_text(text: &str, cursor: &CursorPosition) -> Self {
        let lines = text
            .split('\n')
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        Self {
            lines,
            cursor: CursorSnapshot {
                line: cursor.line,
                column: cursor.column,
            },
        }
    }
}

impl From<ViewportState> for ViewportSnapshot {
    fn from(state: ViewportState) -> Self {
        Self {
            top_line: state.top_line,
            height: state.height,
            scroll_x: state.scroll_x,
            width: state.width,
        }
    }
}

impl MinibufferSnapshot {
    pub fn from_system(system: &MinibufferSystem) -> Self {
        let state = system.minibuffer_state();
        Self {
            mode: describe_mode(&state.mode).to_string(),
            prompt: state.prompt.clone(),
            input: state.input.clone(),
            completions: state.completions.clone(),
            message: state.status_message.clone(),
        }
    }
}

impl From<GuiThemeConfig> for GuiThemeSnapshot {
    fn from(config: GuiThemeConfig) -> Self {
        Self {
            app_background: config.app_background,
            app_foreground: config.app_foreground,
            focus_ring: config.focus_ring,
            active_line_background: config.active_line_background,
            cursor_background: config.cursor_background,
            cursor_foreground: config.cursor_foreground,
            minibuffer_border: config.minibuffer_border,
            minibuffer_prompt: config.minibuffer_prompt,
            minibuffer_input: config.minibuffer_input,
            minibuffer_info: config.minibuffer_info,
            minibuffer_error: config.minibuffer_error,
            statusline_border: config.statusline_border,
            statusline_background: config.statusline_background,
            statusline_foreground: config.statusline_foreground,
        }
    }
}

fn describe_mode(mode: &MinibufferMode) -> &'static str {
    use MinibufferMode::*;
    match mode {
        Inactive => "inactive",
        FindFile => "find-file",
        ExecuteCommand => "execute-command",
        EvalExpression => "eval-expression",
        WriteFile => "write-file",
        SwitchBuffer => "switch-buffer",
        KillBuffer => "kill-buffer",
        SaveConfirmation => "save-confirmation",
        ErrorDisplay { .. } => "error",
        InfoDisplay { .. } => "info",
        QueryReplacePattern => "query-replace-pattern",
        QueryReplaceReplacement => "query-replace-replacement",
        GotoLine => "goto-line",
    }
}
