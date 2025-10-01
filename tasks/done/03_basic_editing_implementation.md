# 基本編集機能実装

## タスク概要
MVPの基本テキスト編集機能（挿入、削除、移動）を実装する。

## 目的
- ギャップバッファを活用した編集機能の実装
- Emacsスタイルキーバインドの基本動作実装
- 文字とカーソル位置の正確な管理
- UTF-8文字列操作の安全な実装

## 実装対象
1. **基本カーソル移動**
   - `C-f` (forward-char) - 右移動
   - `C-b` (backward-char) - 左移動
   - `C-n` (next-line) - 下移動
   - `C-p` (previous-line) - 上移動

2. **文字編集操作**
   - 通常文字入力
   - `DEL` (delete-backward-char) - 後方削除
   - `C-d` (delete-char) - 前方削除
   - `RET` (newline) - 改行挿入

3. **カーソル管理**
   - カーソル位置追跡
   - 行末・行頭での適切な動作
   - ファイル先頭・末尾での境界処理

4. **表示更新**
   - 編集後の画面再描画
   - カーソル位置の視覚的表示
   - スクロール基盤（行単位）

## テスト実装
1. **単体テスト**
   - 各編集操作の正常動作
   - 境界値での動作確認
   - UTF-8マルチバイト文字対応

2. **統合テスト**
   - キーバインド→編集→表示の一連動作
   - 複数操作の組み合わせテスト

## 成果物
- `app/src/editor/mod.rs`
- `app/src/editor/cursor.rs`
- `app/src/editor/actions.rs`
- `app/tests/editing_tests.rs`

## 前提条件
- 02_gap_buffer_implementation.md の完了
- 03_keybinding_design.md の完了
- 01_project_structure_setup.md の完了

## 完了条件
- [x] 基本カーソル移動の実装完成
- [x] 文字編集操作の実装完成
- [x] UTF-8文字対応の確認
- [x] 全テストの成功
- [x] ドキュメントコメントの完備

## 見積もり
**期間**: 3-4日
**優先度**: 最高（コア機能）

## 関連タスク
- 02_gap_buffer_implementation.md（前提）
- 04_tui_interface_implementation.md（統合先）

## 技術的考慮事項
- カーソル位置とギャップバッファ位置の同期
- 行境界での適切なカーソル移動
- アンドゥ機能への将来対応考慮
- パフォーマンス最適化（大きなファイル対応）
