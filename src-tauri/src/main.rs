#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use altre_tauri::{
    BackendController, BackendOptions, BackendResult, EditorSnapshot, KeySequencePayload,
    SaveResponse,
};
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

struct BackendState {
    controller: Mutex<BackendController>,
}

impl BackendState {
    fn try_new(options: BackendOptions) -> BackendResult<Self> {
        let controller = BackendController::new(options)?;
        Ok(Self {
            controller: Mutex::new(controller),
        })
    }

    fn with_controller<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut BackendController) -> BackendResult<T>,
    {
        let mut guard = self
            .controller
            .lock()
            .map_err(|_| "バックエンドのロックに失敗しました".to_string())?;
        f(&mut guard).map_err(|err| err.to_string())
    }
}

fn backend_options_from_env() -> BackendOptions {
    let debug_log_path = env::var_os("ALTRE_GUI_DEBUG_LOG").map(PathBuf::from);
    BackendOptions { debug_log_path }
}

#[tauri::command]
fn editor_snapshot(state: State<BackendState>) -> Result<EditorSnapshot, String> {
    state.with_controller(|controller| controller.snapshot())
}

#[tauri::command]
fn editor_handle_keys(
    state: State<BackendState>,
    payload: KeySequencePayload,
) -> Result<EditorSnapshot, String> {
    state.with_controller(|controller| controller.handle_serialized_keys(payload))
}

#[tauri::command]
fn editor_open_file(state: State<BackendState>, path: String) -> Result<EditorSnapshot, String> {
    state.with_controller(|controller| controller.open_file(&path))
}

#[tauri::command]
fn editor_save_file(state: State<BackendState>) -> Result<SaveResponse, String> {
    state.with_controller(|controller| controller.save_active_buffer())
}

#[tauri::command]
fn editor_shutdown(state: State<BackendState>) -> Result<(), String> {
    state.with_controller(|controller| {
        controller.shutdown();
        Ok(())
    })
}

fn main() {
    let options = backend_options_from_env();
    let state = match BackendState::try_new(options) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("Tauri バックエンドの初期化に失敗しました: {err}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            editor_snapshot,
            editor_handle_keys,
            editor_open_file,
            editor_save_file,
            editor_shutdown
        ])
        .run(tauri::generate_context!())
        .expect("failed to run altre-tauri-app");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn snapshot_command_returns_initial_state() {
        let temp = tempdir().unwrap();
        let options = BackendOptions {
            debug_log_path: Some(temp.path().join("log.jsonl")),
        };
        let state = BackendState::try_new(options).expect("バックエンド初期化に失敗しました");

        let snapshot = state
            .with_controller(|controller| controller.snapshot())
            .expect("スナップショット取得に失敗しました");
        assert_eq!(snapshot.buffer.cursor.line, 0);
    }

    #[test]
    fn open_file_and_save_via_commands() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("log.jsonl");
        let file_path = temp_dir.path().join("sample.txt");
        let options = BackendOptions {
            debug_log_path: Some(log_path),
        };
        let state = BackendState::try_new(options).unwrap();

        state
            .with_controller(|controller| controller.open_file(file_path.to_str().unwrap()))
            .expect("ファイルオープンに失敗しました");

        state
            .with_controller(|controller| {
                controller.handle_serialized_keys(KeySequencePayload {
                    keys: vec![altre_tauri::KeyStrokePayload {
                        key: "a".into(),
                        ctrl: false,
                        alt: false,
                        shift: false,
                    }],
                })
            })
            .expect("キー入力に失敗しました");

        let response = state
            .with_controller(|controller| controller.save_active_buffer())
            .expect("保存に失敗しました");

        assert!(response.success);
        let content = fs::read_to_string(file_path).expect("保存ファイルの読み込みに失敗しました");
        assert_eq!(content, "a");
    }
}
