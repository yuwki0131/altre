# 作業ログ（2025-03-15）

## 実施内容
- GUI 実行時のデバッグ基盤を整備
  - `GuiRunOptions` と `GuiApplication::with_options` を導入し、`--gui`, `--tui`, `--gui-debug`, `--debug-log` の CLI オプションで起動モードとログ出力先を制御できるよう変更。
  - GUI 起動中のキーイベントおよび描画状態を JSON Lines 形式で記録する `DebugLogger` を実装し、`debug/debug.log` へ出力。
  - `C-x` 等の Ctrl 系入力が正しく `KeyEvent::Char` として伝わるようキーコード変換ロジックを更新。
- GUI 初期フォーカスを EditorView に合わせるため、開始時にフォーカスを明示的にセット。
- `nix-shell nix/shell.nix --run "cargo run --features gui -- --gui-debug debug/debug.log"` にて GUI を起動し、ログ生成を確認。

## 得られたログの概要
- `debug/debug.log` は JSON Lines 形式 (`record_type: message/event/state`) で出力。
- ファイル読み込み手順（`C-x C-f` など）やミニバッファ状態の変化が記録され、操作確認が可能。
- XDG Portal 未起動環境では `Error watching for xdg color schemes` が警告として出力されるが、動作・ログ取得には影響なし。

## 今後の検討事項
- `AI_DEBUG_GUIDE.md` に記載された `--test` フラグ実装は未対応。必要に応じて CLI モードでの自動テストを追加する。
- ログレコード数が多いため、必要ならサンプリングやフィルタリングオプションを検討する。

## 追記（2025-03-15）
- Slint ベースの GUI 実装を撤去し、当面は TUI のみを提供する構成に統合。
- `GuiApplication` / `GuiRunOptions` / `--gui` 系 CLI オプションを削除し、`cargo run` で常に TUI が起動するよう変更。
- Slint 依存（`slint`, `slint-build`, `.slint` ファイル、`build.rs`）を整理し、ドキュメントにも Tauri への移行予定である旨を追記。
- GUI 関連ドキュメント（ADR/設計資料/タスク）は履歴として残しつつ、Tauri 再設計が必要である注記を追加。
- ワークスペース構成へ移行し、`altre-core/`（TUI クレート）と `altre-tauri/`（Tauri エントリ用プレースホルダ）を追加。
- `frontend/react/` 配下に React + Vite 雛形を配置し、GUI 実装タスクの受け皿を整備。
- README に TUI/GUI の実行コマンドと NixOS 向け `npm install` / `npm run dev` 手順を追記し、開発手順を整理。
- NixOS 環境（ネットワーク制限下）で `npm install` を試行したが、タイムアウトおよび `ENOTCACHED` エラーにより依存取得できず。ネットワーク許可が必要。
- `altre-tauri` にバックエンド制御モジュール（`BackendController`）とスナップショット/キー変換/デバッグログ基盤を追加。`cargo check` でワークスペース全体がビルド可能なことを確認。
- ユーザー側で依存取得後、`frontend/react` で `npm run build` が成功することを確認。
