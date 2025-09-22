//! ミニバッファ単体テスト
//!
//! 文字入力、カーソル移動、削除の動作確認
//! C-g でのキャンセル処理と状態リセット
//! Unicode文字・制御キー入力の取り扱い

use super::*;
use altre::minibuffer::MinibufferMode;

#[test]
fn test_basic_character_input() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 基本的な文字入力
    helper.simulate_input("test.txt").unwrap();

    assert_eq!(helper.state().input, "test.txt");
    assert_eq!(helper.state().cursor_pos, 8);
}

#[test]
fn test_unicode_character_input() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // Unicode文字の入力テスト
    for test_string in unicode_test_strings() {
        helper.system.deactivate(); // リセット
        helper.start_find_file().unwrap();

        helper.simulate_input(test_string).unwrap();
        assert_eq!(helper.state().input, test_string);

        // カーソル位置は文字数と一致する必要がある
        let expected_pos = test_string.chars().count();
        assert_eq!(helper.state().cursor_pos, expected_pos);
    }
}

#[test]
fn test_cursor_movement() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // テキストを入力
    helper.simulate_input("hello").unwrap();
    assert_eq!(helper.state().cursor_pos, 5);

    // 左矢印キーでカーソル移動
    let left_keys = vec![
        Key::Code(KeyCode::Left),
        Key::Code(KeyCode::Left),
    ];
    helper.simulate_keys(&left_keys);

    // カーソルが移動していることを確認
    assert!(helper.state().cursor_pos < 5);
}

#[test]
fn test_backspace_deletion() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // テキストを入力
    helper.simulate_input("hello").unwrap();
    assert_eq!(helper.state().input, "hello");

    // バックスペースで文字削除
    helper.simulate_backspace().unwrap();
    assert_eq!(helper.state().input, "hell");
    assert_eq!(helper.state().cursor_pos, 4);

    // 複数回バックスペース
    helper.simulate_backspace().unwrap();
    helper.simulate_backspace().unwrap();
    assert_eq!(helper.state().input, "he");
    assert_eq!(helper.state().cursor_pos, 2);
}

#[test]
fn test_delete_key() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // テキストを入力
    helper.simulate_input("hello").unwrap();

    // カーソルを先頭に移動
    let keys = vec![Key::Code(KeyCode::Home)];
    helper.simulate_keys(&keys);

    // Deleteキーで文字削除
    let delete_key = Key::Code(KeyCode::Delete);
    helper.system.handle_key_input(delete_key).unwrap();

    // 最初の文字が削除されているはず
    assert_eq!(helper.state().input, "ello");
}

#[test]
fn test_cancel_with_ctrl_g() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // テキストを入力
    helper.simulate_input("some input").unwrap();
    assert_eq!(helper.state().input, "some input");
    assert!(matches!(helper.state().mode, MinibufferMode::FindFile));

    // C-gでキャンセル
    helper.simulate_cancel().unwrap();

    // 状態がリセットされ、非アクティブになる
    assert!(matches!(helper.state().mode, MinibufferMode::Inactive));
    assert!(helper.state().input.is_empty());
    assert_eq!(helper.state().cursor_pos, 0);
}

#[test]
fn test_input_state_preservation() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 入力とカーソル位置の設定
    helper.simulate_input("test_file.txt").unwrap();
    let original_input = helper.state().input.clone();
    let original_pos = helper.state().cursor_pos;

    // 左矢印でカーソル移動
    let keys = vec![Key::Code(KeyCode::Left), Key::Code(KeyCode::Left)];
    helper.simulate_keys(&keys);

    // 入力内容は保持され、カーソル位置のみ変更
    assert_eq!(helper.state().input, original_input);
    assert!(helper.state().cursor_pos < original_pos);
}

#[test]
fn test_control_character_handling() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 制御文字の入力（通常は無視される）
    let control_chars = vec![
        Key::Ctrl('a'), // home
        Key::Ctrl('e'), // end
        Key::Ctrl('f'), // forward
        Key::Ctrl('b'), // backward
    ];

    helper.simulate_input("test").unwrap();
    let original_input = helper.state().input.clone();

    // 制御文字を送信
    helper.simulate_keys(&control_chars);

    // 入力内容は変更されない（カーソル移動のみ）
    assert_eq!(helper.state().input, original_input);
}

#[test]
fn test_empty_input_handling() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 空の状態でバックスペース
    helper.simulate_backspace().unwrap();
    assert!(helper.state().input.is_empty());
    assert_eq!(helper.state().cursor_pos, 0);

    // 空の状態でEnter
    helper.simulate_enter().unwrap();
    // エラーメッセージが表示されるか、何も起こらない
    // 実装によって動作が異なる可能性がある
}

#[test]
fn test_long_input_handling() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 長いパス名の入力
    let long_path = long_path_test();
    helper.simulate_input(&long_path).unwrap();

    assert_eq!(helper.state().input, long_path);
    assert_eq!(helper.state().cursor_pos, long_path.chars().count());
}

#[test]
fn test_special_characters_in_path() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 特殊文字を含むパス
    let special_paths = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.with.dots.txt",
        "file(with)parentheses.txt",
        "file[with]brackets.txt",
    ];

    for path in special_paths {
        helper.system.deactivate(); // リセット
        helper.start_find_file().unwrap();

        helper.simulate_input(path).unwrap();
        assert_eq!(helper.state().input, path);
    }
}

#[test]
fn test_input_mode_switching() {
    let mut helper = MinibufferTestHelper::new();

    // Find-fileモードで開始
    helper.start_find_file().unwrap();
    assert!(matches!(helper.state().mode, MinibufferMode::FindFile));

    // キャンセルして非アクティブに
    helper.simulate_cancel().unwrap();
    assert!(matches!(helper.state().mode, MinibufferMode::Inactive));

    // Execute-commandモードで開始
    helper.start_execute_command().unwrap();
    assert!(matches!(helper.state().mode, MinibufferMode::ExecuteCommand));

    // 再度キャンセル
    helper.simulate_cancel().unwrap();
    assert!(matches!(helper.state().mode, MinibufferMode::Inactive));
}

#[test]
fn test_input_validation_basic() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_find_file().unwrap();

    // 有効なファイル名パターン
    let valid_names = vec!["test.txt", "README.md", "config.json", "script.sh"];

    for name in valid_names {
        helper.system.deactivate();
        helper.start_find_file().unwrap();

        helper.simulate_input(name).unwrap();
        assert_eq!(helper.state().input, name);

        // エンターキーでの完了をテスト
        helper.simulate_enter().unwrap();
        // 実際のファイル操作は行わず、状態変更のみ確認
    }
}