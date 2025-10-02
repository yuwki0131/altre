# altre

<p align="center">
  <img src="logo/altre-logo-readme.png" alt="altre logo" width="200" />
</p>

Rustとratatuiで構築されたEmacs風テキストエディタです。

## 概要
altreはEmacsの操作モデルを参考に、Rustでゼロから実装しているテキストエディタです。ミニバッファと独自Lisp方言（alisp）による拡張性を備え、ターミナル上で高速に動作する編集体験を目指しています。

## 主な特徴
- Emacs由来のバッファ/ポイント/マークやリージョン操作モデル
- ratatui + crosstermによるTUIとRust製ギャップバッファでの高速編集
- 将来的な拡張を想定したカスタム言語alispと柔軟なキーバインド
- すべてのドキュメントとコメントを日本語で整理

## 現在のステータス
- 開発フェーズ: MVP機能の実装と検証を進行中
- 対応プラットフォーム: TUI（ratatui）を対象、将来GUI拡張を検討
- 開発体制: 個人プロジェクトのため外部からのIssue/PR受付は想定していません

## セットアップ
### 前提条件
- Rust 1.78以降（`rustup`経由のインストールを推奨）
- 色表示に対応した端末（raw modeを利用します）

### ビルド・テスト
```bash
cargo build --offline
cargo test --offline
```

### 実行
```bash
cargo run --offline
```
raw modeが利用できない環境ではTUIが正しく起動しない場合があります。

## 基本操作
- 文字入力とEnter/Tabでテキストを挿入
- Backspace/Deleteで削除
- 矢印キー／Home／End／PageUp／PageDownで移動
- `Ctrl+Q` または `Ctrl+C` で終了
- Emacs風ショートカット（例: `C-x C-f`, `C-x C-s`, `C-x C-c`）を順次実装予定

## リポジトリ構成
```
.
├── app/              # Rustクレート本体
├── docs/             # 設計ドキュメント
├── manuals/          # 利用者・開発者向けマニュアル
├── tasks/            # タスク管理用Markdown
├── AGENTS.md         # エージェント向け開発ガイド
└── README.md         # プロジェクト概要
```

## ドキュメント
- `AGENTS.md`: 開発ガイドラインとノウハウ
- `docs/design/`: 各種設計資料と仕様書
- `QA.md`: 仕様に関するQ&Aと決定事項
- `TASK_MANAGEMENT.md`: タスク運用ルール
- `manuals/`: ユーザー向け・開発者向けマニュアル

## ライセンス
未定（決定次第更新します）。
