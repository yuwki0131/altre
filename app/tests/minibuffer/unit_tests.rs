//! ミニバッファ単体テスト
//!
//! 文字入力・カーソル操作・削除・キャンセル・エラーメッセージの基本挙動を検証

use super::{
    is_continue, key_char, key_plain, key_ctrl, long_path_input, unicode_samples,
    MinibufferMode, MinibufferTestHelper,
};
use altre::input::keybinding::KeyCode;

#[test]
fn test_basic_ascii_input_updates_buffer() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);

    helper.type_text("test.txt");

    assert_eq!(helper.input(), "test.txt");
    assert_eq!(helper.state().cursor_pos, 8);
    assert!(matches!(helper.mode(), MinibufferMode::FindFile));
}

#[test]
fn test_unicode_input_preserves_cursor_position() {
    let mut helper = MinibufferTestHelper::new();

    for sample in unicode_samples() {
        helper.start_find_file(None);
        helper.type_text(sample);

        assert_eq!(helper.input(), *sample);
        assert_eq!(helper.state().cursor_pos, sample.chars().count());
        helper.system().deactivate();
    }
}

#[test]
fn test_backspace_and_delete_remove_expected_characters() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("hello");

    // backspace removes last char
    let response = helper.press_backspace();
    assert!(is_continue(&response));
    assert_eq!(helper.input(), "hell");
    assert_eq!(helper.state().cursor_pos, 4);

    // move cursor to start and delete removes next char
    helper.press_ctrl('a');
    let response = helper.press_delete();
    assert!(is_continue(&response));
    assert_eq!(helper.input(), "ell");
}

#[test]
fn test_cursor_movement_with_arrows_and_shortcuts() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("rust");

    helper.press_arrow(KeyCode::Left);
    helper.press_arrow(KeyCode::Left);
    assert_eq!(helper.state().cursor_pos, 2);

    helper.press_ctrl('a');
    assert_eq!(helper.state().cursor_pos, 0);

    helper.press_ctrl('e');
    assert_eq!(helper.state().cursor_pos, 4);
}

#[test]
fn test_cancel_clears_buffer_and_deactivates() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("sample");

    helper.press_ctrl('g');

    assert!(matches!(helper.mode(), MinibufferMode::Inactive));
    assert!(helper.input().is_empty());
    assert_eq!(helper.state().cursor_pos, 0);
}

#[test]
fn test_enter_without_input_shows_error_message() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);

    let response = helper.press_enter();
    assert!(is_continue(&response));

    match helper.mode() {
        MinibufferMode::ErrorDisplay { message, .. } => {
            assert_eq!(message, "No file specified");
        }
        mode => panic!("expected error display mode, got {:?}", mode),
    }

    // 任意のキーでメッセージが消えることを確認
    helper.send_key(key_char('x'));
    assert!(matches!(helper.mode(), MinibufferMode::Inactive));
}

#[test]
fn test_long_input_updates_cursor_and_preserves_content() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);

    let long_input = long_path_input();
    helper.type_text(&long_input);

    assert_eq!(helper.input(), long_input);
    assert_eq!(helper.state().cursor_pos, helper.input().chars().count());
}

#[test]
fn test_non_printable_input_is_ignored() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file(None);
    helper.type_text("abc");

    // 無効なキーコード
    let response = helper.send_key(key_plain(KeyCode::Unknown));
    assert!(is_continue(&response));
    assert_eq!(helper.input(), "abc");
}
