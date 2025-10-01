// keybinding_integration_tests.rs - キーバインド統合テスト

use altre::input::keybinding::{ModernKeyMap, KeyProcessResult, Action, Key, KeyModifiers, KeyCode};
use altre::input::commands::Command;

#[test]
fn test_mark_and_region_keybindings() {
    let mut keymap = ModernKeyMap::new();

    // C-SPC (マーク設定)
    let ctrl_space = Key {
        modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
        code: KeyCode::Char(' '),
    };
    assert_eq!(
        keymap.process_key(ctrl_space),
        KeyProcessResult::Action(Action::SetMark)
    );

    // C-w (リージョンキル)
    let ctrl_w = Key {
        modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
        code: KeyCode::Char('w'),
    };
    assert_eq!(
        keymap.process_key(ctrl_w),
        KeyProcessResult::Action(Action::KillRegion)
    );

    // M-w (リージョンコピー)
    let alt_w = Key {
        modifiers: KeyModifiers { ctrl: false, alt: true, shift: false },
        code: KeyCode::Char('w'),
    };
    assert_eq!(
        keymap.process_key(alt_w),
        KeyProcessResult::Action(Action::CopyRegion)
    );
}

#[test]
fn test_cx_prefix_keybindings() {
    let mut keymap = ModernKeyMap::new();

    // C-x C-x (カーソルとマークの交換)
    keymap.process_key(Key::ctrl_x());
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::Action(Action::ExchangePointAndMark)
    );

    // C-x h (バッファ全選択)
    keymap.process_key(Key::ctrl_x());
    let h_key = Key {
        modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
        code: KeyCode::Char('h'),
    };
    assert_eq!(
        keymap.process_key(h_key),
        KeyProcessResult::Action(Action::MarkBuffer)
    );
}

#[test]
fn test_action_to_command_conversion() {
    // 各アクションが正しいコマンドに変換されることを確認
    assert_eq!(Action::SetMark.to_command(), Some(Command::SetMark));
    assert_eq!(Action::KillRegion.to_command(), Some(Command::KillRegion));
    assert_eq!(Action::CopyRegion.to_command(), Some(Command::CopyRegion));
    assert_eq!(Action::ExchangePointAndMark.to_command(), Some(Command::ExchangePointAndMark));
    assert_eq!(Action::MarkBuffer.to_command(), Some(Command::MarkBuffer));
    assert_eq!(Action::SwitchBuffer.to_command(), Some(Command::SwitchToBuffer));
    assert_eq!(Action::KillBuffer.to_command(), Some(Command::KillBuffer));
    assert_eq!(Action::ListBuffers.to_command(), Some(Command::ListBuffers));
}

#[test]
fn test_existing_kill_ring_keybindings_still_work() {
    let mut keymap = ModernKeyMap::new();

    // C-k (行キル) - 既存機能が動作することを確認
    assert_eq!(
        keymap.process_key(Key::ctrl_k()),
        KeyProcessResult::Action(Action::KillLine)
    );

    // C-y (ヤンク) - 既存機能が動作することを確認
    assert_eq!(
        keymap.process_key(Key::ctrl_y()),
        KeyProcessResult::Action(Action::Yank)
    );

    // M-y (ヤンクポップ) - 既存機能が動作することを確認
    let alt_y = Key {
        modifiers: KeyModifiers { ctrl: false, alt: true, shift: false },
        code: KeyCode::Char('y'),
    };
    assert_eq!(
        keymap.process_key(alt_y),
        KeyProcessResult::Action(Action::YankPop)
    );
}

#[test]
fn test_keyboard_quit_clears_prefix() {
    let mut keymap = ModernKeyMap::new();

    // C-x プレフィックスを開始
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::PartialMatch
    );

    // C-g でキャンセル（プレフィックス状態でのキャンセル）
    assert_eq!(
        keymap.process_key(Key::ctrl_g()),
        KeyProcessResult::Action(Action::KeyboardQuit)
    );

    // 次のキーがプレフィックス無しで処理されることを確認
    // （ただし、実際にはC-sはNoMatchになる可能性がある）
    let result = keymap.process_key(Key::ctrl_s());
    // プレフィックスがリセットされているので、C-sは単独のキーとして処理される
    // ModernKeyMapでC-sが直接FileSaveに割り当てられているかチェック
    assert!(matches!(result, KeyProcessResult::NoMatch) || matches!(result, KeyProcessResult::Action(Action::FileSave)));
}
