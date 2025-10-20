#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use altre_tauri::{
    BackendController, BackendOptions, BackendResult, EditorSnapshot, KeySequencePayload,
    SaveResponse,
};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State};

struct BackendState {
    options: Mutex<BackendOptions>,
    controller: Mutex<BackendController>,
}

impl BackendState {
    fn try_new(options: BackendOptions) -> BackendResult<Self> {
        let controller = BackendController::new(options.clone())?;
        Ok(Self {
            options: Mutex::new(options),
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

    fn initialize(&self, overrides: BackendOptions) -> Result<EditorSnapshot, String> {
        let mut options_guard = self
            .options
            .lock()
            .map_err(|_| "バックエンドオプションのロックに失敗しました".to_string())?;
        let merged = options_guard.merged_with(&overrides);

        let controller = BackendController::new(merged.clone()).map_err(|err| err.to_string())?;

        let mut controller_guard = self
            .controller
            .lock()
            .map_err(|_| "バックエンドのロックに失敗しました".to_string())?;

        *controller_guard = controller;
        *options_guard = merged;

        controller_guard.snapshot().map_err(|err| err.to_string())
    }
}

fn backend_options_from_env() -> BackendOptions {
    fn env_path(name: &str) -> Option<PathBuf> {
        env::var_os(name).map(PathBuf::from)
    }

    BackendOptions {
        debug_log_path: env_path("ALTRE_GUI_DEBUG_LOG"),
        initial_file: env_path("ALTRE_GUI_INITIAL_FILE"),
        working_directory: env_path("ALTRE_GUI_WORKDIR"),
    }
}

#[tauri::command]
fn editor_snapshot(state: State<BackendState>) -> Result<EditorSnapshot, String> {
    state.with_controller(|controller| controller.snapshot())
}

#[tauri::command]
fn editor_handle_keys(
    app: AppHandle,
    state: State<BackendState>,
    payload: KeySequencePayload,
) -> Result<EditorSnapshot, String> {
    let (snapshot, should_exit) = state.with_controller(|controller| {
        let snapshot = controller.handle_serialized_keys(payload)?;
        let should_exit = !controller.is_running();
        Ok((snapshot, should_exit))
    })?;

    if should_exit {
        app.exit(0);
    }

    Ok(snapshot)
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

#[derive(Debug, Deserialize)]
struct EditorInitRequest {
    #[serde(default)]
    debug_log_path: Option<String>,
    #[serde(default)]
    initial_file: Option<String>,
    #[serde(default)]
    working_directory: Option<String>,
}

impl EditorInitRequest {
    fn into_options(self) -> BackendOptions {
        BackendOptions {
            debug_log_path: to_pathbuf(self.debug_log_path),
            initial_file: to_pathbuf(self.initial_file),
            working_directory: to_pathbuf(self.working_directory),
        }
    }
}

fn to_pathbuf(value: Option<String>) -> Option<PathBuf> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    })
}

#[tauri::command]
fn editor_init(
    state: State<BackendState>,
    request: EditorInitRequest,
) -> Result<EditorSnapshot, String> {
    state.initialize(request.into_options())
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
            editor_init,
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
            ..Default::default()
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
            ..Default::default()
        };
        let state = BackendState::try_new(options).unwrap();

        state
            .with_controller(|controller| controller.open_file(file_path.to_str().unwrap()))
            .expect("ファイルオープンに失敗しました");

        state
            .with_controller(|controller| {
                controller.handle_serialized_keys(KeySequencePayload::from_strokes(vec![
                    altre_tauri::KeyStrokePayload {
                        key: "a".into(),
                        ctrl: false,
                        alt: false,
                        shift: false,
                    },
                ]))
            })
            .expect("キー入力に失敗しました");

        let response = state
            .with_controller(|controller| controller.save_active_buffer())
            .expect("保存に失敗しました");

        assert!(response.success);
        let content = fs::read_to_string(file_path).expect("保存ファイルの読み込みに失敗しました");
        assert_eq!(content, "a");
    }

    #[test]
    fn editor_init_reconfigures_backend() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("log.jsonl");
        let init_file = temp_dir.path().join("init.txt");
        fs::write(&init_file, "initial data\nsecond line")
            .expect("初期ファイルの作成に失敗しました");

        let base_options = BackendOptions {
            debug_log_path: Some(log_path),
            ..Default::default()
        };
        let state =
            BackendState::try_new(base_options).expect("初期バックエンド生成に失敗しました");

        let overrides = BackendOptions {
            initial_file: Some(init_file.clone()),
            working_directory: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        let snapshot = state
            .initialize(overrides)
            .expect("editor_init の適用に失敗しました");

        assert!(
            snapshot.status.label.contains("init.txt"),
            "ステータスラベルが初期ファイル名を含んでいません: {}",
            snapshot.status.label
        );
        assert_eq!(snapshot.buffer.lines[0], "initial data");
    }
}
