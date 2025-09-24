# AGENTガイドライン

## 基本方針（README.mdより抜粋）
- altre は Rust と ratatui で実装する Emacs 風テキストエディタ。バッファ/ポイント/マーク等の概念とミニバッファ、alisp 拡張言語を中心に据える。
- MVP は TUI を対象。将来 GUI (Tauri) などに拡張予定。
- すべてのドキュメントは日本語で記述する方針。**以後のやりとりは必ず日本語で行うこと。**

## リポジトリ構成
- `app/src/` が Rust 実装のルート。主なモジュール:
  - `buffer/` ギャップバッファ・ナビゲーション等
  - `editor/` テキスト編集操作
  - `file/` ファイルI/O・補完
  - `input/` キーバインド・コマンド処理
  - `minibuffer/` ミニバッファシステム（`system.rs` が入口）
  - `ui/` レイアウト・レンダラー・テーマ
  - `alisp/` 初期版 alisp インタプリタ（`docs/design/alisp_language_spec.md` / `docs/design/alisp_runtime_architecture.md` 参照）
- `app/tests/` に統合テスト・性能テスト、`app/benches/` に Criterion ベンチ。
- 設計資料は `docs/design/`、QA 決定は `QA.md` に集約。
- タスク管理は `tasks/` 以下の Markdown を移動させて扱う（`TASK_MANAGEMENT.md` 参照）。

## ビルド・テストコマンド
- `cargo run` : TUI を起動。sandbox や CI では raw mode が拒否される場合があるので注意。
- `cargo test` : 単体・統合・性能テストを実行。デバッグビルドでは `navigation_tab_width_conversion_under_half_millisecond` が `ignore` 扱い。
- `cargo bench --offline` : ギャップバッファ等の性能計測。
- `cargo fmt` / `cargo clippy` : フォーマットと静的解析。

## コーディング指針
- `snake_case` / `PascalCase` を徹底し、公開 API には `//!` / `///` ドキュメントコメントを付与。
- モジュール越しに参照する場合は `pub use` で明示的に再エクスポート。
- alisp まわりは `docs/design/alisp_language_spec.md` と `docs/design/alisp_runtime_architecture.md` の仕様に従う。M-:（`eval-expression`）からミニバッファ経由で評価される。
- 設計や QA の決定（タブ幅 4 固定、性能目標など）は `docs/design/*` と `QA.md` を参照して反映。

## テスト方針
- 各モジュールに `#[cfg(test)]` の単体テストを配置し、結合テストは `app/tests/` へ。
- プロパティテストには `proptest` を利用。失敗再現時は `PROPTEST_CASE_SEED` を記録。
- ナビゲーション性能テストはリリースビルドで閾値を満たす想定。デバッグ環境で数値がばらつく場合は `--release` での確認を推奨。

## タスク運用
- 作業開始時に対象タスクを `tasks/todo/**` から `tasks/proceeding/` へ移動し、完了後は `tasks/done/` へ。
- README や進捗表を更新する場合はタスク記録と整合させる。

## コミュニケーション
- **必ず日本語でやりとりすること。**
- 仕様不明点や判断が必要な場合は `QA.md` にエントリを追加し、回答を得てから着手する。
