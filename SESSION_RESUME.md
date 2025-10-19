# セッション復帰メモ（2025-03-15）

## 現状
- ワークスペースは `altre-core`（TUI）と `altre-tauri`（Rustバックエンド）＋ `src-tauri/`（Tauriアプリ）で構成。
- React UI（`frontend/react`）は `npm run build` 済み。`npm run dev` でブラウザプレビュー、`cargo tauri dev --manifest-path src-tauri/Cargo.toml` でウィンドウ起動（ネットワーク必須 / 現状 fallback UI）。
- バックエンド連携はスタブ段階。`BackendController` の API を Tauri コマンドに接続する作業が未完。

## 未完タスク
1. `tasks/todo/future/tauri_backend_integration.md`
   - Tauri `invoke` コマンドに `BackendController` を接続し、ファイル操作・キー入力・保存等を実装する。
   - `cargo test` 等で API を検証。
2. `tasks/todo/future/tauri_gui_documentation_update.md`
   - README/INSTALL/manuals/AI_DEBUG_GUIDE に Tauri 手順と制限事項を反映。CHANGELOG 更新。
3. `tasks/todo/future/gui_documentation_update.md`（必要に応じて）
4. `tasks/todo/future/gui_regression_tests_implementation.md`（将来的な自動化）

## 次の開始ポイント
- `npm run build --prefix frontend/react` と `cargo tauri dev --manifest-path src-tauri/Cargo.toml` が完了済みなら、`src-tauri/src/main.rs` のスタブコマンドを `altre_tauri::BackendController` に置き換えていく。ネットワーク許可後に `tauri dev` を実行して動作確認。
- ドキュメント更新は GUI 実装の進捗に合わせて `README`/`INSTALL` の Tauri セクションをさらに詳細化する。

## 参考
- `docs/design/tauri_gui_architecture.md`：API・責務分担・ログ仕様。
- `docs/design/tauri_gui_validation.md`：動作確認手順、 fallback テストケース。
