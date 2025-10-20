# Tauri GUI 最小往復フローマイルストーン

## タスク概要
React フロントエンドと Rust バックエンド間で「キー入力→状態更新→スナップショット描画」の往復を成立させる。`docs/design/tauri_gui_minimal_flow.md` の T1/T2/F1/F2 を完了条件とする。

## 作業項目
- [x] **T1** `editor_init` の追加と `BackendOptions` 拡張  
      - `ALTRE_GUI_DEBUG_LOG` に加え、初期バッファ・既定パスの指定を受け付ける。  
      - `tauri::command` 側に初期化エラーの伝搬を追加。
- [x] **T2** 複合キー対応とエラーハンドリング  
      - `KeySequencePayload` を `Vec<KeyStrokePayload>` から `Vec<Vec<KeyStrokePayload>>` などへ拡張する設計案を固める。  
      - `BackendController` に失敗時のエラーメッセージ整形を追加。
- [x] **F1** React 側キー入力ハンドラの複合キー対応  
      - `useEditor` にキー履歴バッファとタイムアウト処理を導入。  
      - IME 入力中は `invoke` を抑制し、`IME 処理中` メッセージをミニバッファ表示に切り替える。
- [x] **F2** スナップショット適用とエラーメッセージ統合  
      - `services/backend.ts` のフォールバック処理をオプトアウト可能にし、実機起動時は常に `invoke` を優先。  
      - スナップショット更新後に UI へ差分反映するロジックと、失敗時にミニバッファへエラーメッセージを表示する仕組みを整える。

## 完了条件
- `cargo tauri dev` で GUI を起動し、`C-x C-f` → ファイル選択 → 編集 → `C-x C-s` が一連の流れで機能する。  
- `npm --prefix frontend/react run build`、`cargo check -p altre-tauri-app` の両方が成功する。  
- 実装内容を `docs/design/tauri_gui_architecture.md` / `log-status.md` に反映済み。

## 備考
- タスク完了後は push 型イベント（差分更新）タスクを検討し、`tasks/todo/future/tauri_gui_validation_plan.md` の内容を更新する。
