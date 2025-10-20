# Tauri GUI アーキテクチャ設計

## 1. 目的
- Rust 製バックエンド（`altre-core` クレート）を Tauri 経由で GUI 化する基本構造を定義する。
- React ベースの Web フロントエンドと Rust バックエンド間のデータフロー・API を明確化する。
- 既存 TUI 実装と共存させるための責務分担と抽象化方針を整理する。

## 2. 全体構成
```
┌──────────────────────────┐
│ altre-tauri (Binary)     │
│  ├─ main.rs              │ ← Tauri::Builder 初期化
│  ├─ state::BackendState  │ ← altre-core::Backend を保持
│  ├─ commands::*          │ ← フロントエンドからの invoke 呼び出し
│  └─ events::*            │ ← emit_all で UI へ通知（将来）
├──────────────────────────┤
│ altre-core (Library)     │
│  ├─ core::Backend        │ ← 編集エンジン（既存）
│  ├─ frontend::Facade     │ ← TUI/GUI 共通の薄いアダプタ（新設）
│  └─ logging::DebugSink   │ ← ~/.altre-log/debug.log 出力
├──────────────────────────┤
│ frontend/react (Web UI)  │
│  ├─ App.tsx              │ ← 画面レイアウト・状態管理
│  ├─ services/backend.ts  │ ← invoke 関数ラッパ / pull 型更新
│  └─ hooks/useEditor.ts   │ ← スナップショット取得と入力送信
└──────────────────────────┘
```

## 3. 責務分担
- **altre-core**: 既存のテキスト編集ロジックを維持。GUI 用に追加するのは `frontend::Facade` レイヤのみ。入力・描画状態のシリアライズ、デバッグログ出力、スナップショット生成を担う。
- **altre-tauri**: Tauri ランタイム初期化、`Backend` の共有、`invoke` コマンド実装、ログオプション処理。
- **React フロントエンド**: UI 描画、キー入力捕捉、バックエンドとの同期。ロジックは最小限に留め、Emacs 風操作はバックエンドに委譲。

## 4. コマンド / API 仕様

### 4.1 現状実装 (`src-tauri/src/main.rs`)

| コマンド名 | リクエスト | レスポンス | 説明 |
|------------|------------|------------|------|
| `editor_init` | `{ debug_log_path?, initial_file?, working_directory? }` | `EditorSnapshot` | GUI 起動時の初期化。オプションを反映したバックエンドを再生成し、最新スナップショットを返す。 |
| `editor_snapshot` | なし | `EditorSnapshot` | バックエンドの最新状態を取得する Pull API。起動直後の初期状態取得にも利用。 |
| `editor_handle_keys` | `{ payload: KeySequencePayload }` | `EditorSnapshot` | キーシーケンスを処理した直後の状態を返す。 |
| `editor_open_file` | `{ path: string }` | `EditorSnapshot` | 指定パスのファイルを開き、アクティブバッファを切り替える。 |
| `editor_save_file` | なし | `SaveResponse` | アクティブバッファを保存し、成功可否とメッセージを返す。 |
| `editor_shutdown` | なし | `()` | バックエンドを明示的に終了。GUI 終了時の後片付けに利用。 |

- `KeySequencePayload` は `[[{ "key": "x", "ctrl": true }], [{ "key": "f", "ctrl": true }]]` のように **時系列順のチャンク配列** で表現する。各チャンクは同時に発生したキー群（現状は1要素）を表す。
- GUI 実行時は Tauri 側の `invoke` 失敗をそのままエラーとして返し、React でミニバッファ／エラーバナーへ表示する。ブラウザプレビュー（`npm run dev`）時のみフォールバックサンプルを利用する。
- `SaveResponse` は `{ success: bool, message: Option<String> }`。

### 4.2 追加予定のコマンド

- `editor_get_snapshot` : Pull 更新を明示的に分離する場合に導入。`editor_handle_keys` は `()` を返し、クライアント側で別途スナップショットを取得する構成も想定。
- `editor_show_dialog` 系 : Tauri 標準のダイアログと連携する場合に、Rust から UI へ指示を出すためのイベントコマンドを検討。

### 4.3 `EditorSnapshot` の構造

現状の `altre-tauri/src/snapshot.rs` は単一バッファに特化した以下の構造を提供している。

```json
{
  "buffer": {
    "lines": ["Hello", "World"],
    "cursor": { "line": 1, "column": 5 }
  },
  "minibuffer": {
    "mode": "find-file",
    "prompt": "Find file:",
    "input": "README.md",
    "completions": [],
    "message": null
  },
  "status": {
    "label": "README.md",
    "isModified": false
  }
}
```

- `buffers` 配列は未実装。複数ウィンドウやバッファ一覧の表示は将来拡張で対応する。
- `render_metadata.status_label` を `status.label` に伝播しているため、TUI 側と表示フォーマットを共有できる。
- 将来の差分更新を見据え、`buffer.lines` は `Vec<String>` のままとし、差分イベントでは `LineUpdate`（要設計）を送る方針。

## 5. 状態管理と更新
- **初期実装**: Pull 型。React 側で操作後に `editor_get_snapshot` を呼び状態を更新。
- React 側では 160ms のフラッシュ遅延を設け、押下されたキーを `KeySequencePayload.sequence` に蓄積してからまとめて送信する。
- **将来拡張**: バックエンドで差分イベントを生成し、`tauri::Window::emit_all` 経由で push 通知。イベント名 `altre://backend-updated` などを想定。
- React 側では Zustand もしくは React Context を使用し、`EditorSnapshot` をアプリ全体で共有する。

## 6. ログ出力
- CLI オプション `--debug-log <path>` を `altre-tauri` 側で受け取り、デフォルトは `~/.altre-log/debug.log`。
- `DebugLog` モジュールは JSON Lines 形式で入出力イベントを記録。例:
  ```jsonl
  {"ts":"...","type":"command","name":"editor_handle_keys","sequence":"C-x C-f"}
  {"ts":"...","type":"snapshot","cursor":{"line":10,"column":0}}
  ```
- TUI と共通化するため、`altre-core::logging::DebugLogger` を抽象化し、 `feature = "gui-debug"` で有効化。

## 7. ディレクトリ構成案
```
.
├── altre-core/          # 既存 TUI クレート
│   └── src/frontend/
│       ├── tui/         # 既存
│       └── facades/tauri.rs  # GUI 用アダプタ（新設）
├── altre-tauri/
│   ├── src/main.rs
│   ├── src/state.rs
│   ├── src/commands.rs
│   └── src/logging.rs
├── frontend/react/
│   ├── src/App.tsx
│   ├── src/hooks/useEditor.ts
│   ├── src/services/backend.ts
│   └── src/styles/
└── debug/               # ログ出力先ディレクトリ（`.gitignore` 済）
```

## 8. TUI 共存方針
- `altre-core` に TUI/GUI 共通の `FrontendFacade` トレイトを定義し、TUI は既存実装、GUI は新アダプタで実装。
- テストでは `cargo test -p altre` で従来通り TUI のみを検証し、GUI 関連は `altre-tauri` の個別テストでカバー。

## 9. 今後の課題
- Push 型イベント導入時の差分計算コストと、フロントエンド側での差分適用ロジック。
- React フロントエンドでのキーシーケンス正規化（修飾キー、IME との相互作用、リピート処理）と `KeySequencePayload` の仕様確定。
- `editor_snapshot` のマルチバッファ化、ウィンドウ分割情報、ミニバッファ表示切替といった UI 拡張。
- IME / 国際化対応の要否（ブラウザエンジン依存になるため検証が必要）。
- Tauri バンドル時のファイルアクセス権限、サンドボックス対応（macOS notarization etc）。

## 10. 参考タスク
- `tasks/todo/future/tauri_backend_integration.md`
- `tasks/todo/future/tauri_gui_react_ui.md`
- `tasks/todo/future/tauri_gui_validation_plan.md`
