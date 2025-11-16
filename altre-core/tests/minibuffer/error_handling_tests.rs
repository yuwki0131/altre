//! ミニバッファエラーハンドリングテスト
//!
//! エラーメッセージ表示、解除、情報メッセージ、およびエラー型のユーザーメッセージを検証

use super::{key_char, MinibufferMode, MinibufferTestHelper, SystemResponse};
use altre::minibuffer::{MinibufferError, SystemEvent};

#[test]
fn test_show_error_api_switches_to_error_mode() {
    let mut helper = MinibufferTestHelper::new();
    let response = helper
        .system()
        .show_error("Permission denied")
        .expect("error display should succeed");
    assert!(matches!(response, SystemResponse::Continue));

    match helper.mode() {
        MinibufferMode::ErrorDisplay { message, .. } => assert_eq!(message, "Permission denied"),
        other => panic!("expected error display mode, got {:?}", other),
    }
}

#[test]
fn test_error_message_clears_on_next_key_input() {
    let mut helper = MinibufferTestHelper::new();
    helper
        .system()
        .show_error("failure")
        .expect("display error");

    helper.send_key(key_char('a'));
    assert!(matches!(helper.mode(), MinibufferMode::Inactive));
}

#[test]
fn test_show_info_api_sets_info_mode() {
    let mut helper = MinibufferTestHelper::new();
    helper
        .system()
        .show_info("Saved successfully")
        .expect("show info");

    match helper.mode() {
        MinibufferMode::InfoDisplay { message, .. } => assert_eq!(message, "Saved successfully"),
        other => panic!("expected info display mode, got {:?}", other),
    }
}

#[test]
fn test_handle_event_show_error_sets_state() {
    let mut helper = MinibufferTestHelper::new();
    helper
        .system()
        .handle_event(SystemEvent::ShowError("IO failed".into()))
        .expect("handle event");

    match helper.mode() {
        MinibufferMode::ErrorDisplay { message, .. } => assert_eq!(message, "IO failed"),
        other => panic!("expected error display mode, got {:?}", other),
    }
}

#[test]
fn test_execute_command_without_input_shows_error() {
    let mut helper = MinibufferTestHelper::new();
    helper.start_execute_command();

    let response = helper.press_enter();
    assert!(matches!(response, SystemResponse::Continue));

    match helper.mode() {
        MinibufferMode::ErrorDisplay { message, .. } => {
            assert_eq!(message, "No command specified");
        }
        other => panic!("expected error display mode, got {:?}", other),
    }
}

#[test]
fn test_minibuffer_error_user_facing_messages() {
    let not_found = MinibufferError::FileNotFound("missing.txt".into());
    assert_eq!(not_found.user_message(), "ファイルが見つかりません: missing.txt");

    let permission = MinibufferError::PermissionDenied("/root".into());
    assert_eq!(permission.user_message(), "アクセス権限がありません: /root");

    let invalid = MinibufferError::InvalidPath("?".into());
    assert_eq!(invalid.user_message(), "無効なパスです: ?");
}
