// global_cancel_tests.rs - C-g グローバルキャンセル機能のテスト

use altre::input::keybinding::{ModernKeyMap, KeyProcessResult, Action, Key};
use altre::search::{SearchController, SearchDirection};
use altre::buffer::editor::TextEditor;
use altre::editor::EditOperations;

#[test]
fn test_ctrl_g_basic_keybinding() {
    let mut keymap = ModernKeyMap::new();

    // C-g がKeyboardQuitアクションに正しくマップされていることを確認
    assert_eq!(
        keymap.process_key(Key::ctrl_g()),
        KeyProcessResult::Action(Action::KeyboardQuit)
    );
}

#[test]
fn test_ctrl_g_cancels_prefix_sequence() {
    let mut keymap = ModernKeyMap::new();

    // C-x プレフィックスを開始
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::PartialMatch
    );

    // プレフィックス状態であることを確認
    assert!(keymap.is_partial_match());

    // C-g でキャンセル
    assert_eq!(
        keymap.process_key(Key::ctrl_g()),
        KeyProcessResult::Action(Action::KeyboardQuit)
    );

    // プレフィックス状態がクリアされていることを確認
    assert!(!keymap.is_partial_match());
}

#[test]
fn test_ctrl_g_resets_mark() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // マークを設定
    editor.move_cursor_to_char(0).unwrap();
    editor.set_mark();
    editor.move_cursor_to_char(5).unwrap();

    // 選択範囲があることを確認
    assert!(editor.selection_range().is_some());

    // keyboard_quitでマークがクリアされることを確認
    // (実際のAppのkeyboard_quit()メソッドの動作をシミュレート)
    editor.clear_mark();

    // マークがクリアされていることを確認
    assert!(editor.selection_range().is_none());
}

#[test]
fn test_search_controller_cancel() {
    let mut editor = TextEditor::new();
    let mut search = SearchController::new();

    // テキストを準備
    editor.insert_str("Hello World Test").unwrap();
    editor.move_cursor_to_char(0).unwrap();

    // 検索を開始
    search.start(&mut editor, SearchDirection::Forward);
    search.input_char(&mut editor, 'W');
    search.input_char(&mut editor, 'o');
    search.input_char(&mut editor, 'r');
    search.input_char(&mut editor, 'l');
    search.input_char(&mut editor, 'd');
    assert!(search.is_active());

    // C-g でキャンセル
    search.cancel(&mut editor);

    // 検索が非アクティブになることを確認
    assert!(!search.is_active());
}

#[test]
fn test_ctrl_g_multiple_contexts() {
    let mut keymap = ModernKeyMap::new();
    let mut editor = TextEditor::new();
    let mut search = SearchController::new();

    // 複数の状態を同時にテスト
    editor.insert_str("Test text").unwrap();
    editor.set_mark();

    // プレフィックス開始
    keymap.process_key(Key::ctrl_x());

    // 検索開始
    search.start(&mut editor, SearchDirection::Forward);
    search.input_char(&mut editor, 't');
    search.input_char(&mut editor, 'e');
    search.input_char(&mut editor, 'x');
    search.input_char(&mut editor, 't');

    // 全て状態があることを確認
    assert!(keymap.is_partial_match());
    assert!(editor.mark().is_some());
    assert!(search.is_active());

    // C-g で一括キャンセル（実際のアプリではkeyboard_quit()がこれらを処理）
    keymap.reset_partial_match();
    editor.clear_mark();
    search.cancel(&mut editor);

    // 全て状態がクリアされていることを確認
    assert!(!keymap.is_partial_match());
    assert!(editor.mark().is_none());
    assert!(!search.is_active());
}

#[test]
fn test_ctrl_g_in_different_modes() {
    let mut keymap = ModernKeyMap::new();

    // 通常モードでC-g
    assert_eq!(
        keymap.process_key(Key::ctrl_g()),
        KeyProcessResult::Action(Action::KeyboardQuit)
    );

    // プレフィックス中にC-g
    keymap.process_key(Key::ctrl_x());
    assert_eq!(
        keymap.process_key(Key::ctrl_g()),
        KeyProcessResult::Action(Action::KeyboardQuit)
    );

    // 状態がリセットされている
    assert!(!keymap.is_partial_match());
}

#[test]
fn test_ctrl_g_always_available() {
    let mut keymap = ModernKeyMap::new();

    // 様々な状態でC-gが常に利用可能であることを確認

    // 通常状態
    let result = keymap.process_key(Key::ctrl_g());
    assert!(matches!(result, KeyProcessResult::Action(Action::KeyboardQuit)));

    // プレフィックス状態
    keymap.process_key(Key::ctrl_x());
    let result = keymap.process_key(Key::ctrl_g());
    assert!(matches!(result, KeyProcessResult::Action(Action::KeyboardQuit)));

    // 複数回連続
    let result1 = keymap.process_key(Key::ctrl_g());
    let result2 = keymap.process_key(Key::ctrl_g());
    assert!(matches!(result1, KeyProcessResult::Action(Action::KeyboardQuit)));
    assert!(matches!(result2, KeyProcessResult::Action(Action::KeyboardQuit)));
}

#[test]
fn test_keyboard_quit_action_to_command() {
    use altre::input::commands::Command;

    // KeyboardQuitアクションが正しいコマンドに変換されることを確認
    assert_eq!(
        Action::KeyboardQuit.to_command(),
        Some(Command::KeyboardQuit)
    );
}