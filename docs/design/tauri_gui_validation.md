# Tauri GUI 動作確認計画

## 1. 目的と前提
- Tauri ベース GUI の MVP 動作確認を手動テスト中心に実施する。
- 現状はバックエンドがプレースホルダ状態のため、`BackendController` の fallback 動作も含めて検証する。
- 実行環境は NixOS + Wayland + Hyprland を想定し、必要なワークアラウンドを整理する。

## 2. 準備手順
1. 依存取得  
   ```bash
   nix-shell nix/shell.nix          # または `nix develop`
   cd frontend/react
   npm install
   npm run build
   ```
2. Rust バックエンドビルド  
   ```bash
   cd ../..
   cargo build -p altre-tauri
   ```
3. デバッグログディレクトリの確認  
   - デフォルト出力先: `~/.altre-log/debug.log`  
   - ディレクトリが無い場合は自動作成されるが、権限不足が無いか確認する。

## 3. 起動手順
- 現時点では Tauri コマンド未実装のため、プレースホルダ挙動を以下で確認する。
  ```bash
  cargo run -p altre-tauri
  ```
  - 成功時: 標準出力に「Tauri GUI プレースホルダ起動」および現在のバッファ情報が表示される。
  - 失敗時: `BackendController::new` の初期化エラーが出力されるのでログを確認する。
- React UI プレビューを確認する場合:
  ```bash
  cd frontend/react
  npm run dev
  ```
  - ブラウザで `http://localhost:5173` にアクセスし、fallback UI が表示されることを確認する。

## 4. 手動テストシナリオ

| No. | シナリオ | 手順 | 期待結果 |
|-----|----------|------|----------|
| 1 | 初期表示 | `npm run dev` で UI を開く | バッファに案内テキストが表示され、ミニバッファに「Tauri backend 未接続」が表示される |
| 2 | 文字入力 | エディタ領域をフォーカスし、`a`, `b`, `Enter` を入力 | fallback バッファに `ab` と改行が追加され、カーソル位置が更新される |
| 3 | Backspace | `Backspace` を入力 | 直前の文字が削除される |
| 4 | 未知キー | `Ctrl+S` を入力 | fallback メッセージ `[fallback] C-S` が挿入される（保存は未実装） |
| 5 | ファイルパス入力 | 下部フォームに `sample.txt` を入力して `Enter` | fallback メッセージ `[fallback] open-file: sample.txt` が表示される |
| 6 | リロード | `リロード` ボタンを押下 | fallback スナップショットが再取得され、UI が最新状態に更新される |

チェックリストは `log.md` に結果を追記するか、`docs/design/tauri_gui_validation.md` 内にテーブル追加して運用する。

## 5. ログ確認手順
- デバッグログは JSON Lines 形式。例:
  ```json
  {"tag":"key_sequence","ts":1710000000000,"payload":["a","b","Enter"]}
  {"tag":"snapshot","ts":1710000000500,"payload":{"buffer":{"lines":["ab",""],"cursor":{"line":1,"column":0}}}}
  ```
- 主な確認項目:
  - `key_sequence`: 入力されたキー
  - `open_file`: ファイル操作の試行
  - `snapshot`: バッファ状態（カーソル位置、ミニバッファ内容）
- CLI オプション例（今後追加予定）:
  - `--debug-log <path>`: ログ出力先を変更（未実装時はデフォルト）
  - `--no-debug-log`: ログ無効化（将来検討）

## 6. 既知の制限・今後の課題
- 現在はバックエンド未実装のため、すべて fallback 動作。Tauri コマンド実装後に再度テストケースを更新する必要がある。
- Push 型イベントを導入した際は、イベント受信テスト（差分更新）が必要。
- IME 入力、ファイルダイアログなど GUI 特有のテストは後続タスクで調査する。
- `tauri dev` 実行およびバンドル生成時の検証は現時点では対象外。
