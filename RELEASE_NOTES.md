# altre v0.1.0 リリースノート

## 概要
altre は Emacs 風の操作性を持つターミナルテキストエディタです。本リリースは MVP（Minimum Viable Product）版であり、日常的なテキスト編集とファイル操作を TUI 上で快適に行うことを目的としています。

## 主要機能
- Emacs ライクなキーバインド (`C-x C-f`, `C-x C-s`, `M-x`, `M-:`)
- ギャップバッファによる高速なカーソル移動と編集
- ミニバッファでのファイル補完・コマンド実行・保存先指定
- `write-file` コマンドによる別名保存フロー
- ratatui ベースのレイアウトとモードライン表示

## ドキュメント
- `manuals/user_guide.md`: 基本操作ガイド
- `manuals/keybinding_reference.md`: キーバインド一覧
- `manuals/troubleshooting.md`: よくある問題と対処法
- `docs/architecture.md`: システム構成
- `docs/api_reference.md`: API 概要

## 既知の制限
- 単一バッファのみ対応（複数ファイル切り替えは計画中）
- Undo/Redo、検索/置換、マクロなど上級機能は未実装
- Windows ネイティブサポートは未検証（WSL2 経由を推奨）
- 端末リサイズ時のレイアウト再計算が不完全

## 今後のロードマップ
1. 複数バッファ管理とウィンドウ分割
2. Undo/Redo と履歴管理
3. キーバインドのユーザー設定ファイル
4. GUI (Tauri) プロトタイプ
5. alisp の拡張（関数定義、パッケージシステム）

## 配布パッケージ
- Linux (x86_64): `dist/altre-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
- チェックサム: `dist/altre-v0.1.0-x86_64-unknown-linux-gnu.sha256`

## インストール
`INSTALL.md` を参照してください。ソースビルド手順とバイナリ展開手順を記載しています。

## 開発者向けメモ
- `scripts/generate_docs.sh` で API ドキュメントを再生成
- `cargo test` を実行して回帰テストを確認
- リリースタグは `v0.1.0` を予定
