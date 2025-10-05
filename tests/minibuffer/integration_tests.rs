//! ミニバッファ統合テスト
//!
//! find-file / save-buffer / quit フローと初期入力設定の統合挙動を検証

use super::{as_file_operation, is_continue, MinibufferMode, MinibufferTestHelper, SystemResponse};
use altre::minibuffer::FileOperation;

#[test]
fn test_find_file_flow_returns_open_operation() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("document.txt");

    let response = helper.press_enter();
    let file_operation = as_file_operation(response).expect("expected file open operation");
    match file_operation {
        FileOperation::Open(path) => assert_eq!(path, "document.txt"),
        other => panic!("unexpected operation: {:?}", other),
    }

    assert!(matches!(helper.mode(), MinibufferMode::Inactive));
}

#[test]
fn test_save_buffer_flow_returns_save_operation() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_execute_command();
    helper.type_text("save-buffer");

    let response = helper.press_enter();
    match response {
        SystemResponse::FileOperation(FileOperation::Save) => {}
        other => panic!("expected save file operation, got {:?}", other),
    }

    assert!(matches!(helper.mode(), MinibufferMode::Inactive));
}

#[test]
fn test_quit_command_requests_application_shutdown() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_execute_command();
    helper.type_text("quit");

    let response = helper.press_enter();
    assert!(matches!(response, SystemResponse::Quit));
}

#[test]
fn test_custom_command_bubbles_to_executor() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_execute_command();
    helper.type_text("other-command");

    let response = helper.press_enter();
    match response {
        SystemResponse::ExecuteCommand(cmd) => assert_eq!(cmd, "other-command"),
        other => panic!("expected command execution, got {:?}", other),
    }
}

#[test]
fn test_find_file_initial_path_sets_prompt_and_cursor() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(Some("~/projects"));

    assert_eq!(helper.input(), "~/projects");
    assert_eq!(helper.state().cursor_pos, "~/projects".chars().count());
    assert_eq!(helper.state().prompt, "Find file: ");
}

#[test]
fn test_cancelled_find_file_does_not_emit_operation() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("temp.txt");

    let response = helper.press_ctrl('g');
    assert!(is_continue(&response));
    assert!(matches!(helper.mode(), MinibufferMode::Inactive));
}

#[test]
fn test_start_execute_command_resets_input() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("keep");

    helper.start_execute_command();
    assert!(helper.input().is_empty());
    assert_eq!(helper.state().prompt, "M-x ");
}
