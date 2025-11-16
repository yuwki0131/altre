use altre::buffer::CursorPosition;
use altre::core::RenderMetadata;
use altre::minibuffer::{MinibufferMode, MinibufferSystem};
use altre::search::{SearchDirection, SearchStatus, SearchUiState};
use altre::ui::viewport::ViewportState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorSnapshot {
    pub buffer: BufferSnapshot,
    pub minibuffer: MinibufferSnapshot,
    pub status: StatusSnapshot,
    pub viewport: ViewportSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<SearchSnapshot>,
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
pub struct SearchSnapshot {
    pub prompt: String,
    pub pattern: String,
    pub status: String,
    pub current_match: Option<usize>,
    pub total_matches: usize,
    pub wrapped: bool,
    pub message: Option<String>,
    pub direction: String,
}

impl EditorSnapshot {
    pub fn new(
        text: &str,
        cursor: &CursorPosition,
        metadata: &RenderMetadata,
        minibuffer: &MinibufferSystem,
        viewport: ViewportState,
    ) -> Self {
        Self {
            buffer: BufferSnapshot::from_text(text, cursor),
            minibuffer: MinibufferSnapshot::from_system(minibuffer),
            status: StatusSnapshot {
                label: metadata.status_label.clone(),
                is_modified: metadata.is_modified,
            },
            viewport: ViewportSnapshot::from(viewport),
            search: metadata.search_ui.as_ref().map(SearchSnapshot::from_state),
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

impl SearchSnapshot {
    fn from_state(state: &SearchUiState) -> Self {
        Self {
            prompt: state.prompt_label.clone(),
            pattern: state.pattern.clone(),
            status: describe_search_status(state.status).to_string(),
            current_match: state.current_match,
            total_matches: state.total_matches,
            wrapped: state.wrapped,
            message: state.message.clone(),
            direction: describe_search_direction(state.direction).to_string(),
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

fn describe_search_status(status: SearchStatus) -> &'static str {
    match status {
        SearchStatus::Active => "active",
        SearchStatus::NotFound => "not-found",
        SearchStatus::Wrapped => "wrapped",
    }
}

fn describe_search_direction(direction: SearchDirection) -> &'static str {
    match direction {
        SearchDirection::Forward => "forward",
        SearchDirection::Backward => "backward",
    }
}
