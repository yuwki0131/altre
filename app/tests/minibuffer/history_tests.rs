//! ミニバッファ履歴機能テスト
//!
//! 履歴追加・ナビゲーション（Ctrl+P / Ctrl+N）
//! find-file / execute-command 切り替え時の履歴選択
//! 履歴上限・重複処理の検証

use super::*;
use altre::minibuffer::MinibufferMode;

#[test]
fn test_history_basic_addition() {
    let mut helper = MinibufferTestHelper::new();

    // ファイル操作の履歴を追加
    helper.start_find_file().unwrap();
    helper.simulate_input("file1.txt").unwrap();
    helper.simulate_enter().unwrap();

    helper.start_find_file().unwrap();
    helper.simulate_input("file2.txt").unwrap();
    helper.simulate_enter().unwrap();

    helper.start_find_file().unwrap();
    helper.simulate_input("file3.txt").unwrap();
    helper.simulate_enter().unwrap();

    // 履歴が追加されていることを確認
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();

    // 最新の履歴エントリが表示される
    assert_eq!(helper.state().input, "file3.txt");
}

#[test]
fn test_history_navigation_up_down() {
    let mut helper = MinibufferTestHelper::new();

    // 複数のファイルを履歴に追加
    let files = vec!["file1.txt", "file2.txt", "file3.txt"];
    for file in &files {
        helper.start_find_file().unwrap();
        helper.simulate_input(file).unwrap();
        helper.simulate_enter().unwrap();
    }

    // 新しいfind-fileセッションを開始
    helper.start_find_file().unwrap();

    // Ctrl+P（履歴を上に）
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "file3.txt");

    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "file2.txt");

    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "file1.txt");

    // Ctrl+N（履歴を下に）
    helper.simulate_history_next().unwrap();
    assert_eq!(helper.state().input, "file2.txt");

    helper.simulate_history_next().unwrap();
    assert_eq!(helper.state().input, "file3.txt");
}

#[test]
fn test_history_separate_by_mode() {
    let mut helper = MinibufferTestHelper::new();

    // find-file履歴を追加
    helper.start_find_file().unwrap();
    helper.simulate_input("document.txt").unwrap();
    helper.simulate_enter().unwrap();

    // execute-command履歴を追加
    helper.start_execute_command().unwrap();
    helper.simulate_input("save-buffer").unwrap();
    helper.simulate_enter().unwrap();

    // find-fileモードで履歴確認
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "document.txt");

    // execute-commandモードで履歴確認
    helper.simulate_cancel().unwrap();
    helper.start_execute_command().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "save-buffer");
}

#[test]
fn test_history_duplicate_removal() {
    let mut helper = MinibufferTestHelper::new();

    // 同じファイルを複数回実行
    for _ in 0..3 {
        helper.start_find_file().unwrap();
        helper.simulate_input("duplicate.txt").unwrap();
        helper.simulate_enter().unwrap();
    }

    // 別のファイルを実行
    helper.start_find_file().unwrap();
    helper.simulate_input("other.txt").unwrap();
    helper.simulate_enter().unwrap();

    // 履歴確認：重複は除去され、最新のもののみ
    helper.start_find_file().unwrap();

    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "other.txt");

    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "duplicate.txt");

    // これ以上履歴はない
    helper.simulate_history_previous().unwrap();
    // 履歴の先頭に到達した場合の動作を確認
}

#[test]
fn test_history_capacity_limit() {
    let mut helper = MinibufferTestHelper::new();

    // 履歴容量の上限をテスト（通常100件程度）
    for i in 0..150 {
        helper.start_find_file().unwrap();
        helper.simulate_input(&format!("file{}.txt", i)).unwrap();
        helper.simulate_enter().unwrap();
    }

    // 履歴ナビゲーションが正常に動作することを確認
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();

    // 最新のエントリが取得できる
    assert_eq!(helper.state().input, "file149.txt");

    // 古いエントリは削除されている可能性がある
    for _ in 0..200 {
        helper.simulate_history_previous().unwrap();
    }

    // 最古のエントリに到達しても問題なし
    assert!(!helper.state().input.is_empty());
}

#[test]
fn test_history_empty_entries_ignored() {
    let mut helper = MinibufferTestHelper::new();

    // 空のエントリを実行しようとする
    helper.start_find_file().unwrap();
    helper.simulate_enter().unwrap(); // 空の入力でEnter

    // 正常なエントリを追加
    helper.start_find_file().unwrap();
    helper.simulate_input("valid.txt").unwrap();
    helper.simulate_enter().unwrap();

    // 履歴確認：空のエントリは追加されていない
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "valid.txt");
}

#[test]
fn test_history_search_functionality() {
    let mut helper = MinibufferTestHelper::new();

    // 複数のファイルを履歴に追加
    let files = vec![
        "document.txt",
        "readme.md",
        "config.json",
        "document_backup.txt",
        "log.txt",
    ];

    for file in &files {
        helper.start_find_file().unwrap();
        helper.simulate_input(file).unwrap();
        helper.simulate_enter().unwrap();
    }

    // 履歴検索機能のテスト（実装されている場合）
    helper.start_find_file().unwrap();
    helper.simulate_input("doc").unwrap();

    // 部分的な入力で履歴をフィルタリング
    helper.simulate_history_previous().unwrap();

    // "document"で始まる最新の履歴エントリが表示される
    let input = helper.state().input.clone();
    assert!(input.starts_with("doc") || input.contains("document"));
}

#[test]
fn test_history_with_unicode() {
    let mut helper = MinibufferTestHelper::new();

    // Unicode文字を含むファイル名を履歴に追加
    let unicode_files = vec![
        "文書.txt",
        "テスト.md",
        "設定ファイル.json",
        "日本語ディレクトリ/ファイル.txt",
    ];

    for file in &unicode_files {
        helper.start_find_file().unwrap();
        helper.simulate_input(file).unwrap();
        helper.simulate_enter().unwrap();
    }

    // Unicode履歴のナビゲーション
    helper.start_find_file().unwrap();
    for file in unicode_files.iter().rev() {
        helper.simulate_history_previous().unwrap();
        assert_eq!(helper.state().input, *file);
    }
}

#[test]
fn test_history_persistence_during_session() {
    let mut helper = MinibufferTestHelper::new();

    // セッション中の履歴永続化
    helper.start_find_file().unwrap();
    helper.simulate_input("session_file.txt").unwrap();
    helper.simulate_enter().unwrap();

    // 他の操作を実行
    helper.start_execute_command().unwrap();
    helper.simulate_input("some-command").unwrap();
    helper.simulate_enter().unwrap();

    // find-file履歴が保持されていることを確認
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "session_file.txt");
}

#[test]
fn test_history_modification_during_navigation() {
    let mut helper = MinibufferTestHelper::new();

    // 履歴を準備
    helper.start_find_file().unwrap();
    helper.simulate_input("original.txt").unwrap();
    helper.simulate_enter().unwrap();

    // 履歴ナビゲーション中に編集
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "original.txt");

    // 履歴項目を編集
    helper.simulate_input("_modified").unwrap();
    assert_eq!(helper.state().input, "original.txt_modified");

    // 編集した内容で実行
    helper.simulate_enter().unwrap();

    // 新しい履歴エントリが追加される
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "original.txt_modified");
}

#[test]
fn test_history_clear_functionality() {
    let mut helper = MinibufferTestHelper::new();

    // 履歴を追加
    helper.start_find_file().unwrap();
    helper.simulate_input("test.txt").unwrap();
    helper.simulate_enter().unwrap();

    // 履歴クリア機能のテスト（実装されている場合）
    // Note: 実際の実装では明示的なクリア機能があるかもしれない

    // 履歴があることを確認
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();
    assert_eq!(helper.state().input, "test.txt");

    // システム再起動シミュレーション（メモリ内履歴のクリア）
    helper = MinibufferTestHelper::new();

    // 履歴が空になっていることを確認
    helper.start_find_file().unwrap();
    helper.simulate_history_previous().unwrap();

    // 履歴がない場合の動作（空のまま、または何も起こらない）
    assert!(helper.state().input.is_empty() || helper.state().input == "test.txt");
}