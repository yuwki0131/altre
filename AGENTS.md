# AGENTガイドライン

## 基本方針（README.mdより抜粋）
- altre は Rust と ratatui で実装する Emacs 風テキストエディタ。バッファ/ポイント/マーク等の概念とミニバッファ、alisp 拡張言語を中心に据える。
- MVP は TUI を対象。将来 GUI (Tauri) などに拡張予定。
- すべてのドキュメントは日本語で記述する方針。**以後のやりとりは必ず日本語で行うこと。**

## 開発メモ
- 個人による試作プロジェクトのため、外部からの Issue や Pull Request の受付は想定していない。
- ドキュメント、コメント、コミットメッセージを含め日本語で統一する。
- リポジトリはモノレポ構成とし、成果物と設計資料を単一リポジトリで管理する。
- 作業完了後はかならずビルドを行い、target/releaseに実装済みアプリケーションを配置した状態にする。

## リポジトリ構成
- `src/` が Rust 実装のルート。主なモジュール:
  - `buffer/` ギャップバッファ・ナビゲーション等
  - `editor/` テキスト編集操作
  - `file/` ファイルI/O・補完
  - `input/` キーバインド・コマンド処理
  - `minibuffer/` ミニバッファシステム（`system.rs` が入口）
  - `ui/` レイアウト・レンダラー・テーマ
  - `alisp/` 初期版 alisp インタプリタ（`docs/design/alisp_language_spec.md` / `docs/design/alisp_runtime_architecture.md` 参照）
- `tests/` に統合テスト・性能テスト、`benches/` に Criterion ベンチ。
- 設計資料は `docs/design/`、QA 決定は `docs/adr-qa/` に集約。
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
- 設計や QA の決定（タブ幅 4 固定、性能目標など）は `docs/design/*` と `docs/adr-qa/init_QA.md` を参照して反映。

## テスト方針
- 各モジュールに `#[cfg(test)]` の単体テストを配置し、結合テストは `tests/` へ。
- プロパティテストには `proptest` を利用。失敗再現時は `PROPTEST_CASE_SEED` を記録。
- ナビゲーション性能テストはリリースビルドで閾値を満たす想定。デバッグ環境で数値がばらつく場合は `--release` での確認を推奨。

## MVP仕様
### 対象環境
- 主要対象: NixOS + Wayland + Hyprland
- MVPインターフェース: ratatui を用いた TUI
- 将来計画: Tauri による GUI 版

### 技術スタック
- コア言語: Rust（安全性とパフォーマンスを重視）
- TUIフレームワーク: ratatui + crossterm
- テキストモデル: ギャップバッファ（シンプル実装）
- 文字エンコーディング: UTF-8 のみ対応
- 改行コード: LF のみ対応
- テスト戦略: `proptest` を用いたプロパティベーステスト

### 機能セット
#### 必須機能
1. **ファイル操作**
   - `C-x C-f` によるファイルオープン（Tab補完、相対/絶対パス対応）
   - `C-x C-s` による保存
   - 新規ファイル作成サポート
2. **基本編集**
   - 文字入力と削除（Backspace/Delete）
   - 改行（Enter）
3. **ナビゲーション**
   - 矢印キー
   - Emacs風移動 (`C-n`/`C-p`/`C-f`/`C-b`)
4. **インターフェース**
   - ミニバッファでのコマンド実行とメッセージ表示
   - 基本的な色サポート
   - `exit` コマンドまたは `C-x C-c` での終了

#### MVP対象外（将来実装）
- alisp インタプリタと拡張API
- シンタックスハイライト（Tree-sitter）
- LSP統合、マウスサポート、高度な検索
- 多様なエンコーディングや改行コード、設定ファイル

## 開発ロードマップ
- フェーズ1: MVP コア（ギャップバッファ、基本操作、ミニバッファ）
- フェーズ2: 検索・置換、モードライン、入力システム拡充
- フェーズ3: alisp 実装と拡張API公開
- フェーズ4: GUI や高度な言語機能、パフォーマンス最適化

## タスク運用
- 作業開始時は対象タスクを `tasks/todo/**` から `tasks/proceeding/` へ移動し、完了後に `tasks/done/` へ移動する。
- README や進捗表を更新する場合はタスク記録と整合させる。
- 1タスク1Markdownファイル方式を守り、カテゴリフォルダ（mvp/alisp/design/bugs/future）を利用する。

## プロジェクト管理方針
- GitHub の Issue/PR は使用せず、すべてローカルファイルで管理する。
- Git ブランチ・コミットは手動で運用し、進捗はタスクファイルに記録する。
- 開発判断が不明な場合は `docs/adr-qa/init_QA.md` にエントリを追加して結論を文書化する。

## コミュニケーション
- **必ず日本語でやりとりすること。**
- 仕様不明点や判断が必要な場合は `docs/adr-qa/init_QA.md` にエントリを追加し、回答を得てから着手する。

## 参考ドキュメント
- `docs/design/` 配下の設計資料
- `manuals/` のユーザー/開発者マニュアル
- `docs/adr-qa/init_QA.md` および `TASK_MANAGEMENT.md`
