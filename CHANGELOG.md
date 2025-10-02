# altre Changelog（開発メモ）

> 現在このプロジェクトは個人開発中で、正式なリリース版は存在しません。以下は将来バージョンを想定した作業ログのドラフトです。

## [0.1.0]（ドラフト）
### 追加
- Rust + ratatui による TUI エディタ MVP（複数バッファ、ミニバッファ、ファイル操作、基本編集）
- ギャップバッファ実装とナビゲーション API（`app/src/buffer/`）
- `CommandProcessor` を中心としたキーバインド処理とコマンド分配（`app/src/input/`）
- `AdvancedRenderer` によるレイアウト分割とミニバッファ統合描画（`app/src/ui/renderer.rs`）
- ファイル操作のアトミックセーブと LF 正規化処理（`app/src/file/operations.rs`）
- インクリメンタル検索エンジンとハイライト描画（`app/src/search/`）
- ベンチマーク／性能監視基盤（`app/src/performance/` と `app/benches/`）

### 改善
- 未保存バッファの保存導線とエラーハンドリングメッセージを調整
- 設計資料（`docs/design/*`）と実装の差異を逐次同期
- テストハーネスを `cargo test --offline` で再現可能な構成へ整理

### 既知の制限（2025-09 時点）
- Undo/Redo や高度な編集機能は未実装
- alisp ランタイムは構想段階（評価器の雛形のみ）
- GUI 版（Tauri）は未着手
- Windows ネイティブ端末、raw mode 非対応環境での検証は限定的

---
より細かなメモは `tasks/` ディレクトリ内の各タスク Markdown を参照してください。
