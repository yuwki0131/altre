//! ミニバッファ補完テスト
//!
//! パス補完候補生成、タブ補完適用、候補選択移動、候補数上限を検証

use super::{is_continue, MinibufferTestHelper};
use altre::input::keybinding::KeyCode;
use std::path::PathBuf;

#[test]
fn test_path_completion_lists_visible_files_only() {
    let mut helper = MinibufferTestHelper::new();
    let dir = helper.prepare_temp_dir(&[
        PathBuf::from("alpha.txt"),
        PathBuf::from("beta.rs"),
        PathBuf::from("dir/nested.md"),
        PathBuf::from(".hidden"),
    ]);

    helper.start_find_file(None);
    helper.type_text(&format!("{}/a", dir.display()));

    let completions = helper.completions();
    assert!(completions.iter().any(|c| c.contains("alpha.txt")));
    assert!(completions.iter().any(|c| c.contains("dir")));
    assert!(!completions.iter().any(|c| c.contains(".hidden")));
    assert_eq!(helper.selected_completion(), Some(0));
}

#[test]
fn test_tab_completion_applies_first_candidate() {
    let mut helper = MinibufferTestHelper::new();
    let dir = helper.prepare_temp_dir(&[PathBuf::from("apple.rs")]);

    helper.start_find_file(None);
    helper.type_text(&format!("{}/ap", dir.display()));

    let completions_before = helper.completions().to_vec();
    assert_eq!(completions_before.len(), 1);

    let response = helper.press_tab();
    assert!(is_continue(&response));
    assert!(helper.input().ends_with("apple.rs"));
}

#[test]
fn test_completion_navigation_updates_selected_index() {
    let mut helper = MinibufferTestHelper::new();
    let dir = helper.prepare_temp_dir(&[
        PathBuf::from("alpha"),
        PathBuf::from("alpine"),
        PathBuf::from("amber"),
    ]);

    helper.start_find_file(None);
    helper.type_text(&format!("{}/a", dir.display()));
    let initial_len = helper.completions().len();
    assert_eq!(initial_len, 3);
    assert_eq!(helper.selected_completion(), Some(0));

    helper.press_arrow(KeyCode::Down);
    assert_eq!(helper.selected_completion(), Some(1));

    helper.press_arrow(KeyCode::Down);
    assert_eq!(helper.selected_completion(), Some(2));

    helper.press_arrow(KeyCode::Down);
    assert_eq!(helper.selected_completion(), Some(0));

    helper.press_arrow(KeyCode::Up);
    assert_eq!(helper.selected_completion(), Some(2));
}

#[test]
fn test_completion_limits_to_fifty_entries() {
    let mut helper = MinibufferTestHelper::new();
    let files: Vec<PathBuf> = (0..75)
        .map(|i| PathBuf::from(format!("file_{:02}.txt", i)))
        .collect();
    let dir = helper.prepare_temp_dir(&files);

    helper.start_find_file(None);
    helper.type_text(&format!("{}/fi", dir.display()));

    let count = helper.completions().len();
    assert!(count <= 50, "expected at most 50 completions, got {}", count);
}
