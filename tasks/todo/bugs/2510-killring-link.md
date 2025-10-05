# kill ringとOS側のコピー・ペーストが連動していない

## 現状
- `src/editor/kill_ring.rs:1` の KillRing はアプリ内専用で、システムクリップボードへ書き出す処理が存在しない。
- コマンド処理 (`src/app.rs:988`) でも OS クリップボード連携が行われておらず、ヤンク時は内部キルリングのみ参照。
- クリップボード連携ライブラリ（例: `arboard`）は `Cargo.toml:1` に未導入。

## 調査事項
- Emacs の kill-new / kill-append 時の `select-enable-clipboard` の仕様整理。
- Wayland / X11 / macOS / Windows で利用可能なクリップボード API の差異。
- TUI (crossterm) でのクリップボードアクセス可否。

## 次アクション
1. クロスプラットフォーム対応クリップボードクレートを選定し、`app` クレートに導入する。
2. KillRing 操作後にシステムクリップボードへ同期する `ClipboardBridge` を新設し、`App::record_kill` に組み込む。
3. ヤンク操作でシステム側が更新されているか `tests/global_cancel_tests.rs` などに回帰テストを追加。
