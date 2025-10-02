# altre

<p align="center">
  <img src="logo/altre-logo-readme.png" alt="altre logo" width="200" />
</p>

Rust と ratatui で構築された Emacs 風テキストエディタです。

## 概要
altre は Emacs の操作モデルを参考に Rust でゼロから実装しているテキストエディタです。ミニバッファと独自 Lisp 方言（alisp）による拡張性を備え、ターミナル上で高速に動作する編集体験を目指しています。

## 主な特徴
- Emacs 互換のバッファ / ポイント / マークとキルリングを実装し、リージョン編集や単語移動をサポート
- ratatui + crossterm による TUI レイアウトと複数ウィンドウ表示（`C-x 2/3/1/0/o`）
- ミニバッファからのコマンド実行 (`M-x`)、ファイル操作 (`C-x C-f`/`C-x C-s`/`C-x C-w`) と補完
- `C-s` / `C-r` によるインクリメンタル検索と検索中のワード追加 (`C-w`)、`Enter` で確定、`C-g` でキャンセル
- 低レイテンシ重視のギャップバッファとパフォーマンス監視（`app/src/performance/`）

## 現在のステータス
- 開発フェーズ: MVP コア機能に加え、バッファ・ウィンドウ管理と検索機能を統合済み
- 対応プラットフォーム: ターミナル向け TUI（ratatui）。将来的に GUI (Tauri) 拡張を計画
- 開発体制: 個人プロジェクトのため外部からの Issue / PR の受付は想定していません

## セットアップ
### 前提条件
- Rust 1.78 以降（`rustup` 経由のインストールを推奨）
- 色表示と raw mode に対応した端末

### ビルド・テスト
```bash
cd app
cargo build --offline
cargo test --offline
```

### 実行
```bash
cd app
cargo run --offline
```
raw mode が利用できない環境では TUI が正しく起動しない場合があります。その場合は `manuals/troubleshooting.md` を参照してください。

## 基本操作
- 文字入力・Backspace/Delete/Enter/Tab による基本編集、`C-d` で前方削除、`C-k` で行キル
- 移動: 矢印キー、`C-f`/`C-b`/`C-n`/`C-p`、`C-a`/`C-e`、`M-f`/`M-b`、`M-<`/`M->`
- マーク・リージョン: `C-SPC` でマーク設定、`C-w` でキル、`M-w` でコピー、`C-y` でヤンク、`M-y` でヤンクポップ、`C-x h` で全選択
- ファイル: `C-x C-f` で開く、`C-x C-s` で保存、`C-x C-w` で別名保存、`C-x s` で全バッファ保存
- バッファ: `C-x b` で切替、`C-x k` で削除、`C-x C-b` で一覧表示
- ウィンドウ: `C-x 2`/`C-x 3` で分割、`C-x 1` で単一表示、`C-x 0` で閉じる、`C-x o` でフォーカス切替、`C-x <`/`C-x >` で水平スクロール
- 検索: `C-s` / `C-r` で開始、`C-w` で単語追加、`Enter` で確定、`C-g` でキャンセル
- ミニバッファ: `M-x` でコマンド実行、`M-:` で alisp 評価、`C-g` でキャンセル
- 終了: `C-x C-c` で保存確認なしに終了（必要に応じてミニバッファで確認メッセージを表示）

## リポジトリ構成
```
.
├── app/                  # Rust クレート本体
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
- `AGENTS.md`: 開発ガイドラインとノウハウ
- `docs/adr/`: アーキテクチャ決定記録（例: `0004-alisp-first-draft.md`）
- `docs/design/`: 設計資料と仕様書
- `manuals/`: ユーザーガイド・キーバインド一覧・トラブルシューティング
- `QA.md`: 仕様に関する Q&A と決定事項
- `TASK_MANAGEMENT.md`: タスク運用ルール

## ライセンス
未定（決定次第更新します）。
