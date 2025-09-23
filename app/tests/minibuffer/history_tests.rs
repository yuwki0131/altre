//! ミニバッファ履歴テスト
//!
//! 履歴追加、Ctrl+P/Ctrl+N ナビゲーション、重複除去、容量管理を検証

use super::{as_file_operation, MinibufferTestHelper, SystemResponse};
use altre::minibuffer::FileOperation;

fn execute_find_file(helper: &mut MinibufferTestHelper, name: &str) {
    helper.start_find_file(None);
    helper.type_text(name);
    let response = helper.press_enter();
    let operation = as_file_operation(response).expect("expected file operation");
    match operation {
        FileOperation::Open(path) => assert_eq!(path, name),
        other => panic!("unexpected operation: {:?}", other),
    }
}

fn execute_command(helper: &mut MinibufferTestHelper, command: &str) {
    helper.start_execute_command();
    helper.type_text(command);
    let response = helper.press_enter();
    match response {
        super::SystemResponse::ExecuteCommand(actual) => assert_eq!(actual, command),
        other => panic!("unexpected response: {:?}", other),
    }
}

#[test]
fn test_history_records_completed_entries() {
    let mut helper = MinibufferTestHelper::new();
    execute_find_file(&mut helper, "file1.txt");

    helper.start_find_file(None);
    helper.press_ctrl('p');

    assert_eq!(helper.input(), "file1.txt");
    assert_eq!(helper.state().history.len(), 1);
}

#[test]
fn test_history_navigation_order_with_ctrl_p() {
    let mut helper = MinibufferTestHelper::new();
    for name in ["file1.txt", "file2.txt", "file3.txt"] {
        execute_find_file(&mut helper, name);
    }

    helper.start_find_file(None);
    helper.press_ctrl('p');
    assert_eq!(helper.input(), "file3.txt");

    helper.press_ctrl('p');
    assert_eq!(helper.input(), "file2.txt");

    helper.press_ctrl('p');
    assert_eq!(helper.input(), "file1.txt");
}

#[test]
fn test_history_navigation_forward_with_ctrl_n_restores_empty_input() {
    let mut helper = MinibufferTestHelper::new();
    execute_find_file(&mut helper, "project.md");
    execute_find_file(&mut helper, "notes.txt");

    helper.start_find_file(None);
    helper.press_ctrl('p');
    helper.press_ctrl('p');
    assert_eq!(helper.input(), "project.md");

    helper.press_ctrl('n');
    assert_eq!(helper.input(), "notes.txt");

    helper.press_ctrl('n');
    assert!(helper.input().is_empty());
    assert_eq!(helper.state().history_index, None);
}

#[test]
fn test_history_mixed_modes_are_accessible() {
    let mut helper = MinibufferTestHelper::new();
    execute_find_file(&mut helper, "document.txt");
    execute_command(&mut helper, "save-buffer");

    // command モードの履歴
    helper.start_execute_command();
    helper.press_ctrl('p');
    assert_eq!(helper.input(), "save-buffer");

    // find-file モードでも履歴からアクセス出来る
    helper.press_ctrl('g');
    helper.start_find_file(None);
    helper.press_ctrl('p');
    assert_eq!(helper.input(), "save-buffer");
    helper.press_ctrl('p');
    assert_eq!(helper.input(), "document.txt");
}

#[test]
fn test_history_deduplicates_entries_and_respects_capacity() {
    let mut helper = MinibufferTestHelper::new();

    for _ in 0..3 {
        execute_find_file(&mut helper, "duplicate.txt");
    }
    assert_eq!(helper.state().history.len(), 1);

    for index in 0..120 {
        execute_find_file(&mut helper, &format!("file_{index:03}.txt"));
    }

    let history_len = helper.state().history.len();
    assert!(history_len <= 100, "history should cap at 100, got {}", history_len);
}

#[test]
fn test_history_search_via_prefix_input() {
    let mut helper = MinibufferTestHelper::new();
    execute_find_file(&mut helper, "src/main.rs");
    execute_find_file(&mut helper, "src/lib.rs");
    execute_find_file(&mut helper, "README.md");

    let matches = helper.state().history.search("src");
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().all(|(_, entry)| entry.starts_with("src")));
}
