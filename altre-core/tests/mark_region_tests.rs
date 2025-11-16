// mark_region_tests.rs - マーク・リージョン機能のテスト

use altre::buffer::editor::TextEditor;
use altre::editor::EditOperations;

#[test]
fn test_mark_set_and_clear() {
    let mut editor = TextEditor::new();

    // 初期状態ではマークは未設定
    assert_eq!(editor.mark(), None);
    assert_eq!(editor.selection_range(), None);

    // マークを設定
    editor.set_mark();
    assert_eq!(editor.mark(), Some(0));

    // マークをクリア
    editor.clear_mark();
    assert_eq!(editor.mark(), None);
}

#[test]
fn test_selection_range_basic() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // カーソルを先頭に戻してマーク設定
    editor.move_cursor_to_char(0).unwrap();
    editor.set_mark();

    // カーソルを5文字目に移動
    editor.move_cursor_to_char(5).unwrap();

    // 選択範囲を確認（0から5）
    assert_eq!(editor.selection_range(), Some((0, 5)));

    // 選択範囲のテキストを取得
    let selected_text = editor.selection_text().unwrap().unwrap();
    assert_eq!(selected_text, "Hello");
}

#[test]
fn test_selection_range_reverse() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // カーソルが末尾(11)にある状態でマーク設定
    editor.set_mark();

    // カーソルを5文字目に移動
    editor.move_cursor_to_char(5).unwrap();

    // 選択範囲を確認（自動的にソート: 5から11）
    assert_eq!(editor.selection_range(), Some((5, 11)));

    // 選択範囲のテキストを取得
    let selected_text = editor.selection_text().unwrap().unwrap();
    assert_eq!(selected_text, " World");
}

#[test]
fn test_mark_adjustment_on_insert() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // カーソルを5文字目に移動してマーク設定
    editor.move_cursor_to_char(5).unwrap();
    editor.set_mark();

    // カーソルを先頭に戻して文字挿入
    editor.move_cursor_to_char(0).unwrap();
    editor.insert_char('X').unwrap();

    // マークが1つ右にシフトしていることを確認
    assert_eq!(editor.mark(), Some(6));
}

#[test]
fn test_mark_adjustment_on_delete() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // カーソルを5文字目に移動してマーク設定
    editor.move_cursor_to_char(5).unwrap();
    editor.set_mark();

    // カーソルを先頭に戻して1文字削除
    editor.move_cursor_to_char(1).unwrap();
    editor.delete_backward().unwrap();

    // マークが1つ左にシフトしていることを確認
    assert_eq!(editor.mark(), Some(4));
}

#[test]
fn test_swap_cursor_and_mark() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // カーソルを先頭に移動してマーク設定
    editor.move_cursor_to_char(0).unwrap();
    editor.set_mark();

    // カーソルを5文字目に移動
    editor.move_cursor_to_char(5).unwrap();

    // カーソルとマークを交換
    editor.swap_cursor_and_mark().unwrap();

    // カーソルが0、マークが5になっていることを確認
    assert_eq!(editor.cursor().char_pos, 0);
    assert_eq!(editor.mark(), Some(5));
}

#[test]
fn test_mark_entire_buffer() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello\nWorld\n123").unwrap();

    // カーソルを中央に移動
    editor.move_cursor_to_char(7).unwrap();

    // バッファ全体を選択
    editor.mark_entire_buffer().unwrap();

    // 文字数を確認（"Hello\nWorld\n123" = 15文字）
    let buffer_len = editor.to_string().chars().count();

    // マークが先頭(0)、カーソルが末尾(buffer_len)に設定されていることを確認
    assert_eq!(editor.mark(), Some(0));
    assert_eq!(editor.cursor().char_pos, buffer_len);

    // 選択範囲がバッファ全体になっていることを確認
    assert_eq!(editor.selection_range(), Some((0, buffer_len)));
}

#[test]
fn test_delete_range_span() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // カーソルを先頭に移動してマーク設定
    editor.move_cursor_to_char(0).unwrap();
    editor.set_mark();

    // カーソルを5文字目に移動
    editor.move_cursor_to_char(5).unwrap();

    // 範囲を削除
    let deleted_text = editor.delete_range_span(0, 5).unwrap();
    assert_eq!(deleted_text, "Hello");

    // 残りのテキストを確認
    assert_eq!(editor.to_string(), " World");
}

#[test]
fn test_position_to_line_column() {
    let mut editor = TextEditor::new();

    // 複数行テキストを挿入
    editor.insert_str("Hello\nWorld\n123").unwrap();

    // 各位置の行・列を確認
    assert_eq!(editor.position_to_line_column(0), (0, 0)); // H
    assert_eq!(editor.position_to_line_column(5), (0, 5)); // \n
    assert_eq!(editor.position_to_line_column(6), (1, 0)); // W
    assert_eq!(editor.position_to_line_column(11), (1, 5)); // \n
    assert_eq!(editor.position_to_line_column(12), (2, 0)); // 1
    assert_eq!(editor.position_to_line_column(13), (2, 1)); // 末尾
}

#[test]
fn test_mark_clamp_on_buffer_changes() {
    let mut editor = TextEditor::new();

    // テキストを挿入
    editor.insert_str("Hello World").unwrap();

    // マークを末尾に設定
    editor.set_mark();
    assert_eq!(editor.mark(), Some(11));

    // バッファを短くする（範囲削除）
    editor.delete_range_span(5, 11).unwrap();

    // マークが適切にクランプされていることを確認
    assert_eq!(editor.mark(), Some(5));
    assert_eq!(editor.to_string(), "Hello");
}
