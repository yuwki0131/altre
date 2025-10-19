# AI_DEBUG_GUIDE.md
> **目的:**  
> Codex CLI や Claude Code などの AI エージェントが、ターミナル/ツールベースの環境から altre を安全にデバッグ・検証するためのガイドラインを提供します。

---

## 現状（2025-03-15 時点）

- altre は ratatui + crossterm による **TUI 実装のみが有効** です。  
- 旧 Slint ベース GUI フロントエンドは除去済みで、**Tauri を用いた新 GUI 設計を再検討中** です。  
- `--gui` / `--tui` / `--gui-debug` / `--debug-log` といった CLI フラグは廃止され、`cargo run` は常に TUI を起動します。  
- GUI 専用のログ出力 (`debug/debug.log` など) は生成されません。必要に応じて一般的な Rust ログ (`RUST_LOG`) を利用してください。

---

## CLI ベース検証フロー

1. **ビルド検証**  
   ```bash
   cargo check
   ```
   依存解決を行わずに型チェックのみが行われます。ネットワーク制限のある環境では `--offline` を付与してください。

2. **テスト実行**  
   ```bash
   cargo test
   ```
   単体テスト・結合テストをまとめて実行します。パフォーマンス測定を行う場合は `cargo bench --offline` を使用します。

3. **実行確認（TUI）**  
   ```bash
   cargo run --offline
   ```
   TUI を起動し、基本的な編集操作（ファイル読み込み、移動、検索、Undo/Redo など）を確認します。raw mode を利用できない環境では起動に失敗する可能性があるため、その場合は `manuals/troubleshooting.md` を参照してください。

---

## ログとデバッグ

- 重大なランタイムエラーは `anyhow::Result` と panic ハンドラで捕捉され、CLI 上に表示されます。  
- 追加のログが必要な場合は `RUST_LOG=debug cargo run` のように環境変数で制御してください。  
- 旧 GUI デバッグロガーは撤去済みです。GUI ログの再導入は Tauri 実装タスクで再検討します。

---

## 今後の Tauri 移行に向けて

- GUI フロントエンドの再設計方針が固まり次第、Tauri ベースでの自動テスト/デバッグ手順を本ガイドに追記します。  
- それまでは TUI 部分の品質確保を優先し、GUI 関連タスクは `tasks/` 配下の該当ファイルで進捗管理してください。
