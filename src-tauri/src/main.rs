#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CursorSnapshot {
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BufferSnapshot {
    lines: Vec<String>,
    cursor: CursorSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinibufferSnapshot {
    mode: String,
    prompt: String,
    input: String,
    completions: Vec<String>,
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusSnapshot {
    label: String,
    is_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditorSnapshot {
    buffer: BufferSnapshot,
    minibuffer: MinibufferSnapshot,
    status: StatusSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyStrokePayload {
    key: String,
    ctrl: Option<bool>,
    alt: Option<bool>,
    shift: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeySequencePayload {
    keys: Vec<KeyStrokePayload>,
}

#[tauri::command]
fn editor_snapshot() -> EditorSnapshot {
    fallback_snapshot()
}

#[tauri::command]
fn editor_handle_keys(payload: KeySequencePayload) -> EditorSnapshot {
    let mut snapshot = fallback_snapshot();
    for stroke in payload.keys {
        if stroke.ctrl.unwrap_or(false) || stroke.alt.unwrap_or(false) {
            snapshot
                .buffer
                .lines
                .push(format!("[fallback] {}", format_key(&stroke)));
            continue;
        }

        match stroke.key.as_str() {
            "Enter" => snapshot.buffer.lines.push(String::new()),
            "Backspace" => {
                if let Some(last) = snapshot.buffer.lines.last_mut() {
                    if !last.is_empty() {
                        last.pop();
                    }
                }
            }
            key if key.len() == 1 => {
                if let Some(last) = snapshot.buffer.lines.last_mut() {
                    last.push_str(key);
                }
            }
            _ => snapshot
                .buffer
                .lines
                .push(format!("[fallback] {}", format_key(&stroke))),
        }
    }
    snapshot
}

#[tauri::command]
fn editor_open_file(path: String) -> EditorSnapshot {
    let mut snapshot = fallback_snapshot();
    snapshot
        .buffer
        .lines
        .push(format!("[fallback] open-file: {path}"));
    snapshot
}

fn format_key(stroke: &KeyStrokePayload) -> String {
    let mut parts = Vec::new();
    if stroke.ctrl.unwrap_or(false) {
        parts.push("C".to_string());
    }
    if stroke.alt.unwrap_or(false) {
        parts.push("M".to_string());
    }
    if stroke.shift.unwrap_or(false) {
        parts.push("S".to_string());
    }
    parts.push(stroke.key.clone());
    parts.join("-")
}

fn fallback_snapshot() -> EditorSnapshot {
    EditorSnapshot {
        buffer: BufferSnapshot {
            lines: vec![
                "Tauri GUI は準備中です。".into(),
                "Rust バックエンドと接続できないため、ローカルサンプルを表示しています。".into(),
                "依存が揃ったら Tauri コマンドを実装し、invoke() が成功するようにしてください。".into(),
            ],
            cursor: CursorSnapshot { line: 2, column: 24 },
        },
        minibuffer: MinibufferSnapshot {
            mode: "inactive".into(),
            prompt: "M-x".into(),
            input: String::new(),
            completions: Vec::new(),
            message: Some("Tauri backend 未接続 (fallback)".into()),
        },
        status: StatusSnapshot {
            label: "scratch (fallback)".into(),
            is_modified: false,
        },
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            editor_snapshot,
            editor_handle_keys,
            editor_open_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run altre-tauri-app");
}
