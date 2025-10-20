# Tauri GUI 最小往復フロー実装方針

## 1. 背景
Tauri GUI プロトタイプは `invoke` コマンドと React フロントエンドを用意済みだが、以下の理由で完全な往復フロー（入力→バックエンド→スナップショット→描画）が未完成である。

- `altre-tauri` のコマンド群は Pull 型スナップショットを返すのみで、初期化やエラー挙動の仕様が明文化されていない。
- React 側の `services/backend.ts` は Tauri ランタイム未接続時のフォールバックに重点を置いており、実機接続時のエラーハンドリング・更新タイミングが暫定的。
- キーシーケンスの正規化が 1 ストローク単位に限定されており、`C-x C-f` など複合キーを扱う拡張が必要。
- バックエンドでのスナップショット生成 (`EditorSnapshot`) が単一バッファ前提のため、TUI レイアウトとの差異が残っている。

本ドキュメントはこれらのギャップを最小構成で解消し、Tauri GUI を「編集操作が往復し画面表示が更新される」状態に持っていくための作業指針を整理する。

## 2. 基本シーケンス

```
React(UI) ── keydown ──▶ useEditor.handleKeyDown
  │                         │
  │                         ▼
  │                 services/backend.sendKeySequence
  │                         │  (invoke)
  │                         ▼
  │                 tauri::command `editor_handle_keys`
  │                         │
  │                         ▼
  │                BackendController.handle_serialized_keys
  │                         │
  │                         ▼
  │                 Backend.snapshot() / EditorSnapshot
  │                         │
  ▼                         ▼
React 状態更新 ◀─────────── JSON 応答
```

Pull 更新は以下のタイミングで実行する。

1. アプリ起動時 (`useEffect` 初回) に `editor_snapshot` を呼び初期状態を取得。
2. キーシーケンス送信後に応答スナップショットで状態を更新。
3. ファイルオープン／保存など副作用を伴う操作後に `editor_snapshot` を再取得し差分を吸収。
4. 明示リロード（UI ボタン）時は `editor_snapshot` を再度呼び出す。

## 3. バックエンド側の最小タスク

1. **初期化 API の整備**  
   - `editor_snapshot` 呼び出し時に未初期化の場合は自動初期化する現行実装を残しつつ、`editor_init` を追加して将来のオプション拡張を可能にする。  
   - `BackendOptions` に初期バッファや既定ディレクトリなど GUI 固有の設定を追加できる余地を作る。

2. **キーシーケンスの複合化対応**  
   - `KeySequencePayload` を `Vec<Vec<KeyStrokePayload>>` 形式へ拡張し、時系列チャンク単位でバックエンドへ渡す。  
   - エラーレスポンス時に `AltreError::Application` で整形し、React 側でメッセージ表示に利用する。

3. **スナップショットの拡張余地を確保**  
   - `EditorSnapshot` に `buffer_id` や `window_layout`（暫定 `Option<String>` など）を追加して互換性を保ちつつ将来拡張のフックを設ける。  
   - JSON シリアライズが余計な usize → number 変換を起こさないよう `serde(with = "...")` を必要に応じて検討。

4. **テスト整備**  
   - `src-tauri/src/main.rs` のユニットテストに複合キー入力 (`C-x`, `C-f`) の往復を追加。  
   - `altre-tauri` 側にも `KeySequencePayload` → `Command` 変換のテストケースを増やす。

## 4. フロントエンド側の最小タスク

1. **キー入力ハンドリング強化**  
   - `useEditor` にキー履歴バッファとタイムアウト（既定 160ms）を導入し、複数キーをまとめて送信。  
   - IME 使用時は `event.isComposing` を検知してバックエンド送信を抑制。

2. **スナップショット適用**  
   - `useEditor` で `EditorSnapshot` を `setState` 経由で保持し、React 再描画で `buffer.lines` を更新（初期段階では全更新で許容）。  
   - エラーメッセージは状態として保持し、ミニバッファ領域とエラーバナーへ表示。`isTauriRuntime()` が false の場合のみフォールバック描画を使用する。

3. **ファイル操作 UI**  
   - ヘッダーに `開く…` ボタンを追加し、Tauri ランタイムでは `__TAURI_INTERNALS__.dialog.open` を優先、未提供の場合は `window.prompt` にフォールバックしてパスを取得する。  
   - `保存` ボタンを追加し、`editor_save_file` のレスポンス（成功可否とメッセージ）をミニバッファへ反映する。

4. **動作確認ツール**  
   - `npm run dev` を Tauri 起動なしでも使用できるよう、`fallback` と `mockSnapshot` の切り替えスイッチを導入。  
   - スクリーンショット取得や簡易 E2E（Playwright 検討）に備え、初期表示コマンドを定義。

## 5. タスク整理

| 項番 | 種別 | 内容 | 優先度 | 担当候補 |
|------|------|------|--------|----------|
| T1 | Rust | `editor_init` 追加と `BackendOptions` 拡張 | 高 | core |
| T2 | Rust | `handle_serialized_keys` の複合キー対応とエラーハンドリング | 高 | core |
| T3 | Rust | `EditorSnapshot` に将来拡張用フィールドを追加、テスト強化 | 中 | core |
| F1 | React | キー入力ハンドラの複合キー対応（タイムアウト/IME 検知） | 高 | frontend |
| F2 | React | スナップショット適用ロジック整備とエラーメッセージ統合 | 高 | frontend |
| F3 | React | ファイル操作 UI 改善（ダイアログ抽象、保存メッセージ） | 中 | frontend |
| F4 | 共通 | `fallback` 表示→設定で無効化できる仕組み追加 | 低 | frontend |
| QA1 | テスト | `cargo tauri dev` 向け E2E 手順と自動化検討 | 中 | shared |

これらのタスクは `tasks/todo/future/` 配下に個別ファイルを作成し、完了後は `tasks/done/` へ移動する。まずは T1/T2/F1/F2 をまとめて「最小往復フロー」マイルストーンとして扱う。

## 6. 依存関係
- T1/T2 は `altre-core` のキーマップ・バッファ操作に影響するため、既存 TUI テストが通ることを前提に実施する。
- F1/F2 は React 側の状態管理刷新が絡むため、`services/backend.ts` の API 変更が伴う場合はバージョン管理に注意する。
- `npm --prefix frontend/react run build` と `cargo tauri dev` を定常的に実行できる環境（Nix シェル + GUI 依存）が整っていることが前提。

## 7. 成果物
- 本ドキュメントの内容をベースに、各タスクファイルへチェックリストと完了条件を記述。
- 実装完了後は `docs/design/tauri_gui_architecture.md` のコマンド表とスナップショット仕様を更新し、`log-status.md` にビルド確認結果を記録する。
