# MVPテストチェックリスト

## 実施概要
- 実施日: 2025-02-05
- 担当: QA(自動試験 + 手動確認)
- 検証対象: MVPコア機能（バッファ/基本編集/ファイル操作/ミニバッファ/ウィンドウ管理/ナビゲーション性能）

## 自動テスト結果
- `cargo test --offline` を実行し、単体・統合・プロパティテスト 254件 + 統合テスト群が全て成功。
  - 重要モジュール: `app/tests/integration_tests.rs`, `app/tests/keybinding_integration_tests.rs`, `app/tests/navigation_performance.rs` (性能閾値チェックを含む) が成功。
- `cargo bench --offline navigation_bench` を実行し、ナビゲーション系ベンチマークの退行なし (Criterion レポート更新)。

## 手動/体感確認
> Sandbox 環境では `cargo run` による TUI 実行が制限されるため、以下の項目をローカル実行環境で確認する手順と、代替として実施した自動化シナリオを併記する。

| 機能 | 手動確認手順 | 実施状況 |
|------|--------------|----------|
| 基本編集 | `cargo run` → 新規バッファで文字入力/削除/改行 → 期待どおり状態が更新されることを確認 | 代替: `app/tests/integration_tests.rs::test_basic_editing` で確認済み |
| ファイル操作 | `C-x C-f` で既存ファイルを開き `C-x C-s` で保存 | 代替: `app/tests/integration_tests.rs::test_file_operations` で確認済み |
| ナビゲーション | 矢印キー/`C-f`/`C-b`/`C-n`/`C-p` → カーソル移動挙動が仕様通り | 代替: `app/tests/integration_tests.rs::test_cursor_movement` + `app/tests/navigation_performance.rs` |
| ミニバッファ | `M-x` → `write-file` などコマンド呼び出し → メッセージ表示確認 | 代替: `app/tests/extended_file_operations_tests.rs` |
| ウィンドウ管理 | `C-x 2`/`C-x 1` → ウィンドウ分割・復元 | 代替: `app/tests/window_management_tests.rs` |
| Undo/Redo | `C-/`, `C-.` → 編集履歴が正しく戻る | 代替: `app/tests/integration_tests.rs::test_basic_editing` 内で履歴検証 |

※ 実機手動テストは `manuals/user_guide.md` のコマンド一覧に従うことで再現可能。TUI操作結果は別途キャプチャ予定。

## パフォーマンス検証
- `navigation_performance` テストにて以下の閾値を確認済み:
  - カーソル移動: < 1ms (`navigation_basic_motions_under_one_millisecond`)
  - 長行移動: < 5ms (`navigation_long_line_performance_targets`)
  - バッファ全体移動: < 2ms (`navigation_buffer_wide_operations_within_two_milliseconds`)
- Criterion ベンチマーク (`app/benches/navigation_bench.rs`, `app/benches/performance.rs`) を実行し、過去ベースラインからの退行無し。

## ドキュメント整合性チェック
- `README.md` / `manuals/user_guide.md` の操作説明と実装キーバインド (`manuals/keybinding_reference.md`) が一致することを確認。
- 性能要件は `docs/design/performance_tests_spec.md` に記載された閾値と `app/tests/navigation_performance.rs` のアサーションが一致。

## 評価
- 自動テスト: ✅ 完了
- 手動（代替シナリオ含む）: ✅ 完了
- パフォーマンス閾値: ✅ 目標達成（退行なし）
- ドキュメント整合性: ✅ 最新状態に更新済み

## 今後のフォローアップ
- TUI 実機での目視確認を行う場合は、Wayland + Hyprland 環境で `cargo run --offline` を実行し、上記チェックリストを手動で再確認すること。
- 検索/置換機能追加後は `tasks/todo/mvp/22-24` に沿って本チェックリストを拡張予定。
