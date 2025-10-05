# altre リリースノート（ドラフト）

この文書は将来リリースを整理するための個人用メモです。現時点で正式な配布物やバージョン番号は存在しません。

## 現状まとめ（2025-09 時点）
- Emacs 風キーバインドとミニバッファを備えた TUI エディタの MVP が動作
- ファイルオープン／保存、複数バッファ切替、インクリメンタル検索をサポート
- ギャップバッファと `AdvancedRenderer` を活用した高速描画が安定
- パフォーマンスベンチとテストスイートを `cargo test --offline` / `cargo bench --offline` で運用中

## 主要ドキュメント
- `manuals/user_guide.md` : 基本操作メモ
- `manuals/keybinding_reference.md` : キーバインド一覧
- `manuals/troubleshooting.md` : トラブル対応メモ
- `docs/` 以下 : 設計・仕様メモ
- `docs/adr/` : アーキテクチャ決定記録

## 主な未完了事項
- Undo/Redo、検索/置換以外の高機能編集操作
- alisp ランタイム統合と拡張 API 公開
- GUI (Tauri) プロトタイプとプラットフォーム検証
- ライセンス方針、配布チャネルの確定

## 作業メモ
- ビルド・テスト手順は `INSTALL.md` を参照（リポジトリルートで `cargo` を実行）
- バイナリ配布は未実施。利用時はローカルで `cargo build --release --offline` を実行
- 進捗・TODO は `tasks/` ディレクトリの Markdown に記録
