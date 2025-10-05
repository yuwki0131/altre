# altre

<p align="center">
  <img src="logo/altre-logo-readme.png" alt="altre logo" width="200" />
</p>

Emacs風テキストエディタ実装

## 概要
* altre は Emacs の操作モデルを参考にRustでゼロから実装しているテキストエディタ
* 独自の拡張言語としてLisp方言（altre lisp）を採用し、ターミナル上で高速に動作する編集体験を目指す

## 主な現在の実装状況
- Emacs 互換のバッファ / ポイント / マークとキルリングを実装し、リージョン編集や単語移動をサポート
- ratatui + crossterm による TUI レイアウトと複数ウィンドウ表示
- ミニバッファからのコマンド実行 (`M-x`)
- ファイル操作
- インクリメンタル検索
- 線形 Undo / Redo (`C-/`, `C-.`)
- 低レイテンシ重視のギャップバッファとパフォーマンス監視（`src/performance/`）

## 現在のステータス
- 開発フェーズ: MVP コア機能に加え、バッファ・ウィンドウ管理と検索機能を統合済み
- 対応プラットフォーム: ターミナル向け TUI（ratatui）。将来的に GUI (Tauri) 拡張を計画

## セットアップ
### 前提条件
- Rust 1.78 以降（`rustup` 経由のインストールを推奨）
- 色表示と raw mode に対応した端末

### ビルド・テスト

```bash
cargo build --offline
cargo test --offline
```

### 実行

```bash
cargo run --offline
```

* raw mode が利用できない環境では TUI が正しく起動しない場合あり
* トラブルシューティングは`manuals/troubleshooting.md` を参照

## 基本操作
- 文字入力・Backspace/Delete/Enter/Tab による基本編集、`C-d` で前方削除、`C-k` で行キル
- 移動: 矢印キー、`C-f`/`C-b`/`C-n`/`C-p`、`C-a`/`C-e`、`M-f`/`M-b`、`M-<`/`M->`
- マーク・リージョン: `C-SPC` でマーク設定、`C-w` でキル、`M-w` でコピー、`C-y` でヤンク、`M-y` でヤンクポップ、`C-x h` で全選択
- ファイル: `C-x C-f` で開く、`C-x C-s` で保存、`C-x C-w` で別名保存、`C-x s` で全バッファ保存
- バッファ: `C-x b` で切替、`C-x k` で削除、`C-x C-b` で一覧表示
- ウィンドウ: `C-x 2`/`C-x 3` で分割、`C-x 1` で単一表示、`C-x 0` で閉じる、`C-x o` でフォーカス切替、`C-x <`/`C-x >` で水平スクロール
- 検索: `C-s` / `C-r` で開始、`C-w` で単語追加、`Enter` で確定、`C-g` でキャンセル
- Undo / Redo: `C-/` で取り消し、`C-.` / `C-?` / `C-\\` / `C-4` でやり直し
- ミニバッファ: `M-x` でコマンド実行、`M-:` で alisp 評価、`C-g` でキャンセル
- 終了: `C-x C-c` で保存確認なしに終了（必要に応じてミニバッファで確認メッセージを表示）

## リポジトリ構成
```
.
├── benches/              # Criterion ベンチマーク
├── src/                  # Rust クレート本体
├── tests/                # 結合テスト・統合テスト
├── Cargo.toml            # Cargo マニフェスト
├── Cargo.lock            # 依存関係ロックファイル
├── docs/                 # 設計ドキュメント・ADR
│   ├── adr/              # アーキテクチャ決定記録
│   └── design/           # 詳細設計・仕様
├── manuals/              # 利用者・開発者向けマニュアル
├── scripts/              # 開発補助スクリプト
├── tasks/                # タスク管理用 Markdown
├── dist/                 # 配布物（ドラフト）
├── logo/                 # ロゴアセット
├── AGENTS.md             # エージェント向け開発ガイド
├── CHANGELOG.md          # 変更履歴ドラフト
├── DEBUG.md              # デバッグモード手順
├── INSTALL.md            # インストール手順
├── performance_report.md # 性能最適化レポート
└── README.md             # プロジェクト概要
```

## ドキュメント
- `AGENTS.md`: 開発ガイドラインとノウハウ(codex読み込み用)
- `docs/adr/`: アーキテクチャ決定記録（例: `0004-alisp-first-draft.md`）
- `docs/design/`: 設計資料と仕様書
- `manuals/`: ユーザーガイド・キーバインド一覧・トラブルシューティング
- `docs/adr-qa/`: 仕様に関する Q&A と決定事項（例: `docs/adr-qa/init_QA.md`）
- `TASK_MANAGEMENT.md`: タスク運用ルール

## ライセンス
未定
