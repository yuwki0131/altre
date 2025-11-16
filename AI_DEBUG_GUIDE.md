# AI_DEBUG_GUIDE.md
> **目的:**  
> Codex CLI や Claude Code などの AI エージェントが、ターミナル/ツールベースの環境から altre を安全にデバッグ・検証するためのガイドラインを提供します。

---

## 現状（2025-10 時点）

- altre は **Tauri + React ベースの GUI** と **ratatui ベースの TUI** の両モードを提供します。  
- `cargo run -p altre` は GUI を優先して起動し、GUI 依存が不足する場合は自動的に TUI へフォールバックします。  
- `cargo run -p altre -- --gui` / `--tui` でモードを強制指定できます。  
- GUI 起動には Node.js 18 以上・npm・`@tauri-apps/cli` と GTK / WebKitGTK / libsoup などの依存が必要です。`nix-shell nix/shell.nix` で一括準備できます。

---

## 推奨検証フロー

1. **Rust ワークスペースのビルド検証**  
   ```bash
   cargo check
   ```
   依存解決を行わずに型チェックのみが行われます。ネットワーク制限のある環境では `--offline` を付与してください。

2. **テスト実行**  
   ```bash
   cargo test
   ```
   単体テスト・結合テストをまとめて実行します。パフォーマンス測定を行う場合は `cargo bench --offline` を使用します。

3. **GUI 起動確認（Tauri）**  
   ```bash
   nix-shell nix/shell.nix       # もしくは各 OS で GTK/WebKit 依存をセットアップ
   npm install --prefix frontend/react
   npm run build --prefix frontend/react
   cargo run -p altre -- --gui
   ```
   初回は `target/release/altre-tauri-app` を自動ビルドします。GUI 実行後はキー入力・ファイルオープン・保存がバックエンドと往復することを確認してください。ホットリロードは `cargo tauri dev --manifest-path src-tauri/Cargo.toml` で利用できます。

4. **TUI 起動確認**  
   ```bash
   cargo run -p altre -- --tui
   ```
   raw mode を利用できない環境では起動に失敗する可能性があるため、その場合は `manuals/troubleshooting.md` を参照してください。

---

## GUI デバッグとログ

- GUI 実行時は `ALTRE_GUI_DEBUG_LOG=/path/to/log.jsonl cargo run -p altre -- --gui` のように環境変数を指定することで JSON Lines 形式の操作ログを出力できます（既定は `~/.altre-log/debug.log`）。  
- バックエンドは `tauri::State<BackendController>` を通じて共有されており、`editor_*` コマンドを `invoke` してテストできます。  
- ブラウザプレビュー (`npm run dev --prefix frontend/react`) 中は Tauri ランタイムが存在しないため fallback モードで動作し、キー入力はローカルバッファに記録されます。

---

## TUI / 共通デバッグ

- 重大なランタイムエラーは `anyhow::Result` と panic ハンドラで捕捉され、CLI 上に表示されます。  
- 追加ログが必要な場合は `RUST_LOG=debug cargo run -p altre -- --tui` のように環境変数で制御してください。  
- `cargo test -p altre-tauri -- --nocapture` で GUI バックエンドの単体テストをそのまま実行できます（ネットワーク未接続の場合は必要なクレートの事前キャッシュを用意してください）。

---

## 今後の Tauri 移行に向けて

- GUI 差分イベントの push 型配信や回帰テスト自動化は未実装です。`tasks/todo/future/gui_regression_tests_implementation.md` を参照してください。  
- IME やファイルダイアログの自動化テストは今後の課題として整理中です。
