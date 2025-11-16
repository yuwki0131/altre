# セッション復帰メモ（2025-10-20）

## 現状
- ワークスペースは `altre-core`（CLI エントリ / TUI 実装）、`altre-tauri`（GUI バックエンド）、`src-tauri/`（Tauri アプリ）、`frontend/react`（GUI フロントエンド）で分割済み。
- `cargo run -p altre` は GUI を優先して起動し、`target/release/altre-tauri-app` が存在しない場合は自動ビルド。GUI の実行に失敗した場合は TUI へフォールバックする。
- Tauri コマンド (`editor_*`) はすべて `BackendController` に接続済みで、キー入力・ファイルオープン・保存が一往復で反映される。`KeySequencePayload` はチャンク形式に対応。
- React 側は `useEditor` フックで 160ms のキーシーケンスバッファを持ち、IME 判定・エラーバナー表示・ヘッダーからの「開く/保存」操作を実装済み。

## 未完タスク
1. `docs/design/tauri_gui_minimal_flow.md` の T3/F4/QA1 など未着手項目（スナップショット拡張、fallback の切り替え設定、E2E 手順策定）。
2. `tasks/todo/future/tauri_gui_documentation_update.md` と `tasks/todo/future/gui_documentation_update.md` のチェックリスト更新（主要ドキュメントには反映済み、残りは CHANGELOG など）。
3. `tasks/todo/future/gui_regression_tests_implementation.md`：GUI/TUI 共通の自動テスト整備と手動チェックリストの更新。
4. `docs/design/tauri_gui_validation.md` の追記（push イベント導入時のテスト観点、ログ解析テンプレート拡充）。

## 次の開始ポイント
- `nix-shell nix/shell.nix` → `npm install --prefix frontend/react` → `npm run build --prefix frontend/react` → `cargo run -p altre -- --gui` で GUI を再確認。`ALTRE_GUI_DEBUG_LOG` に出力が残ることをチェック。
- CLI でのみ作業する場合は `cargo run -p altre -- --tui` を利用。`cargo test -p altre-tauri` / `-p altre-tauri-app` はネットワークが遮断されている環境では失敗するため、事前に依存キャッシュを準備する。

## 参考
- `docs/design/tauri_gui_architecture.md`：API・責務分担・ログ仕様。
- `docs/design/tauri_gui_minimal_flow.md`：未完タスクと優先度。
- `docs/design/tauri_gui_validation.md`：手動テスト手順、ログ確認方法。
