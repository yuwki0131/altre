// extended_file_operations_tests.rs - 拡張ファイル操作機能のテスト

use altre::input::keybinding::{ModernKeyMap, KeyProcessResult, Action, Key};
use altre::input::commands::{Command, CommandProcessor};
use altre::buffer::EditOperations;

#[test]
fn test_write_file_keybinding() {
    let mut keymap = ModernKeyMap::new();

    // C-x プレフィックスを開始
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::PartialMatch
    );

    // C-w を押してwrite-fileアクションを確認
    assert_eq!(
        keymap.process_key(Key::ctrl_w()),
        KeyProcessResult::Action(Action::WriteFile)
    );
}

#[test]
fn test_save_all_buffers_keybinding() {
    let mut keymap = ModernKeyMap::new();

    // C-x プレフィックスを開始
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::PartialMatch
    );

    // s を押してsave-some-buffersアクションを確認
    assert_eq!(
        keymap.process_key(Key {
            modifiers: altre::input::keybinding::KeyModifiers { ctrl: false, alt: false, shift: false },
            code: altre::input::keybinding::KeyCode::Char('s'),
        }),
        KeyProcessResult::Action(Action::SaveAllBuffers)
    );
}

#[test]
fn test_write_file_action_to_command() {
    // WriteFileアクションが正しいコマンドに変換されることを確認
    assert_eq!(
        Action::WriteFile.to_command(),
        Some(Command::WriteFile)
    );
}

#[test]
fn test_save_all_buffers_action_to_command() {
    // SaveAllBuffersアクションが正しいコマンドに変換されることを確認
    assert_eq!(
        Action::SaveAllBuffers.to_command(),
        Some(Command::SaveAllBuffers)
    );
}

#[test]
fn test_command_from_string() {
    // 文字列からコマンドが正しく作成されることを確認
    assert_eq!(
        Command::from_string("write-file"),
        Command::WriteFile
    );

    assert_eq!(
        Command::from_string("save-some-buffers"),
        Command::SaveAllBuffers
    );
}

#[test]
fn test_command_description() {
    // コマンドの説明が正しく取得できることを確認
    assert_eq!(
        Command::WriteFile.description(),
        "別名でファイルを保存"
    );

    assert_eq!(
        Command::SaveAllBuffers.description(),
        "すべてのバッファを保存"
    );
}

#[test]
fn test_save_all_buffers_execution() {
    let mut processor = CommandProcessor::new();

    // テキストを挿入（mutableな参照が必要）
    // 現在のAPIでは直接テキストを挿入できないため、シンプルなテストに変更

    // save-some-buffersコマンドを実行
    let result = processor.execute(Command::SaveAllBuffers);

    // 現在は単一バッファのみ対応のため、バッファがない場合はエラー
    assert!(!result.success);
    assert!(result.message.is_some());
}

#[test]
fn test_write_file_execution() {
    let mut processor = CommandProcessor::new();

    // write-fileコマンドを実行（ミニバッファ経由である必要がある）
    let result = processor.execute(Command::WriteFile);

    // ミニバッファ経由での実行が必要なため、エラーメッセージが表示される
    assert!(!result.success);
    assert!(result.message.is_some());
    assert!(result.message.unwrap().contains("ミニバッファ経由"));
}

#[test]
fn test_cx_prefix_maintains_state() {
    let mut keymap = ModernKeyMap::new();

    // C-x プレフィックスを開始
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::PartialMatch
    );

    // プレフィックス状態が維持されていることを確認
    assert!(keymap.is_partial_match());

    // C-w でwrite-fileアクション
    let result = keymap.process_key(Key::ctrl_w());
    assert!(matches!(result, KeyProcessResult::Action(Action::WriteFile)));

    // プレフィックス状態がリセットされていることを確認
    assert!(!keymap.is_partial_match());
}

#[test]
fn test_extended_file_operations_commands_available() {
    use altre::minibuffer::commands::CommandProcessor as MinibufferProcessor;

    let processor = MinibufferProcessor::new();

    // 新しいコマンドが利用可能であることを確認
    assert!(processor.command_exists("write-file"));
    assert!(processor.command_exists("save-some-buffers"));

    // エイリアスも確認
    let completions = processor.complete_command("write");
    assert!(!completions.is_empty());
    assert!(completions.iter().any(|c| c.contains("write-file")));
}