# altre アーキテクチャ概要

## 1. システム全体像
altre は TUI ベースの Emacs 風テキストエディタです。Rust 製のバックエンドと `ratatui` による描画層を中心に、以下のレイヤで構成されています。

1. **UI レイヤ** (`src/ui/`)
   - レイアウト計算、レンダリング、テーマ管理を担当
   - `AdvancedRenderer` がメインエントリで、`LayoutEngine` が画面分割を行う
2. **アプリケーションレイヤ** (`src/app.rs`)
   - イベントループ、キーバインドディスパッチ、状態遷移を管理
   - `CommandProcessor`、`MinibufferSystem`、`TextEditor` を統合
3. **ドメインレイヤ**
   - バッファ管理（ギャップバッファ）、ファイル操作、ミニバッファ制御、alisp 実行系
4. **インフラレイヤ**
   - Crossterm による入力イベント、ファイルシステム I/O、設定読み込み

## 2. モジュール構成
| ディレクトリ | 概要 | 主要型 |
|--------------|------|--------|
| `buffer/` | テキストバッファ、カーソル操作、ギャップバッファ実装 | `TextBuffer`, `TextEditor`, `NavigationAction` |
| `editor/` | 編集コマンドの高レベル API | `EditorController` (計画中) |
| `file/` | ファイル入出力、保存、補完 | `FileBuffer`, `FileOperationManager`, `FileSaver` |
| `input/` | キーバインドとコマンド分派 | `ModernKeyMap`, `Command`, `CommandProcessor` |
| `minibuffer/` | ミニバッファ状態と UI | `MinibufferSystem`, `ModernMinibuffer`, `MinibufferMode` |
| `ui/` | レイアウトと描画 | `AdvancedRenderer`, `LayoutContext` |
| `alisp/` | altre Lisp インタプリタ | `Interpreter`, `Value`, `Environment` |

詳細なモジュール依存は `docs/architecture/module_dependencies.md` を参照してください。

## 3. データフロー
1. 入力イベントが `crossterm` から取得され `App::run` のイベントループへ渡る。
2. `ModernKeyMap` がキーシーケンスを解決し `Command` に変換。
3. `CommandProcessor` がコマンドを実行し、必要であれば `FileOperationManager` や `TextEditor` を更新。
4. `App` が結果メッセージを `MinibufferSystem` に表示し、再描画要求を `AdvancedRenderer` に送出。
5. `AdvancedRenderer` が最新の `Editor` 状態とミニバッファ状態を取得し、`ratatui` で UI を描画。

## 4. 設計原則
- **Emacs 互換性優先**: キーバインドとミニバッファフローは Emacs の体験を踏襲。
- **Rust らしさと安全性**: 所有権と借用を活かし、バッファ管理を明確化。
- **拡張性**: 将来の GUI (Tauri) への移行を見据え、UI 層を抽象化。
- **テスト容易性**: 各モジュールに単体テストを配置し、統合テストは `tests/` に集約。

## 5. 依存関係
- `crossterm`: 端末制御とキーボード入力
- `ratatui`: TUI レイアウトと描画
- `serde` / `serde_json`: 設定・補完データ読込（予定）
- `criterion`: ベンチマーク（ギャップバッファ性能計測）

## 6. 将来の拡張ポイント
- バッファの複数管理 (`buffer::Workspace`)
- Undo/Redo スタック
- キーバインドの外部定義とホットリロード
- GUI レンダラーの追加 (`ui/gui/`)

## 7. 関連ドキュメント
- `docs/architecture/mvp_architecture.md`: 詳細なモジュール設計
- `docs/design/minibuffer.md`: ミニバッファ仕様
- `docs/design/file_operations.md`: ファイル操作設計
- `docs/design/alisp_runtime_architecture.md`: alisp 実装

このドキュメントは新規コントリビューター向けのエントリポイントとして位置づけられています。詳細は各モジュールの Rustdoc コメントを参照してください。
