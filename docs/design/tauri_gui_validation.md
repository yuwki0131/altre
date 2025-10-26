# Tauri GUI 動作確認計画

## 1. 目的と前提
- Tauri ベース GUI が「キー入力 → バックエンド処理 → スナップショット反映」の往復フローを完了できることを確認する。
- ファイルオープン／保存など副作用を含む操作が React UI から実行できることを検証する。
- 実行環境は NixOS + Wayland + Hyprland を想定するが、GTK / WebKitGTK / libsoup / Node.js / Tauri CLI が揃っていれば他 OS でも同様に検証可能。

## 2. 準備手順
1. 依存取得  
   ```bash
   nix-shell nix/shell.nix          # または各 OS で GTK / WebKit / libsoup を導入
   npm install --prefix frontend/react
   npm run build --prefix frontend/react
   ```
2. バックエンドビルド  
   ```bash
   cargo build -p altre-tauri
   cargo build -p altre-tauri-app --release   # GUI バイナリ（target/release）を生成
   ```
3. デバッグログ設定（任意）  
   - 既定の出力先: `~/.altre-log/debug.log`  
   - `ALTRE_GUI_DEBUG_LOG=/tmp/altre-gui.log` のように環境変数で出力先を変更可能。

## 3. 起動手順
- 通常起動:  
  ```bash
  cargo run -p altre -- --gui
  ```
  - 初回実行時は `altre-tauri-app` を自動ビルドし GUI を起動する。GUI バイナリが終了コード 0 以外で終了した場合は TUI へフォールバックする。
- 開発モード（ホットリロード）:
  ```bash
  cargo tauri dev --manifest-path src-tauri/Cargo.toml
  ```
  - `frontend/react` をウォッチして自動ビルドしながら起動する。ネットワークアクセスが必須。
- ブラウザプレビューのみ確認したい場合:
  ```bash
  npm run dev --prefix frontend/react
  ```
  - Tauri ランタイムに接続しないため、UI は fallback モードで動作する。

## 4. 手動テストシナリオ

| No. | シナリオ | 手順 | 期待結果 |
|-----|----------|------|----------|
| 1 | 起動確認 | `cargo run -p altre -- --gui` を実行 | GUI ウィンドウが開き、scratch バッファが表示される |
| 2 | 文字入力 | バッファをフォーカスし `a` `b` `Enter` を入力 | バッファに `a` `b` と改行が反映され、カーソル位置が更新される |
| 3 | 複合キー | `Ctrl+N` `Ctrl+P` を入力 | バックエンドの行追加/移動ロジックが適用され、カーソルが上下に移動する |
| 4 | ファイルオープン | ヘッダーの「開く…」からファイルを指定、または `C-x C-f` を送る | バッファが指定ファイルの内容に更新され、ステータスラベルがファイル名を表示する |
| 5 | 保存 | 編集後にヘッダーの「保存」または `C-x C-s` | `SaveResponse.success` が true となり、ミニバッファに成功メッセージが表示され、ファイル内容が更新される |
| 6 | ミニバッファメッセージ | 意図的にエラーを起こす（存在しないパスを開く等） | ミニバッファが `error` モードになり、エラーメッセージが表示される |
| 7 | 終了 | `C-x C-c` を送る、またはウィンドウを閉じる | `backend.is_running()` が false になり、アプリケーションが終了する |

検証結果は `log.md` に追記するか、日付ごとにメモを残しておく。

## 5. ログ確認手順
- `ALTRE_GUI_DEBUG_LOG` を設定した状態で GUI を起動すると、各 `invoke` コマンドとスナップショットが JSON Lines 形式で記録される。例:
  ```json
  {"tag":"key_sequence","ts":1789392000000,"payload":["C-x","C-s"]}
  {"tag":"snapshot","ts":1789392000123,"payload":{"status":{"label":"README.md","is_modified":false}}}
  ```
- 主な確認項目:
  - `key_sequence`: 送信されたキーシーケンス
  - `open_file`: ファイルオープン時のパス
  - `snapshot`: バッファ内容・カーソル位置・ミニバッファ状態
  - `save_buffer`: 保存成否とメッセージ
- ログが肥大化する場合はテキストフィルタや `jq` を併用し、必要なイベントのみ抽出する。

## 6. 既知の制限・今後の課題
- Push 型イベント（`emit_all`）は未実装であり、UI 更新はすべて Pull 型スナップショットに依存している。導入時は差分イベントの検証ケースを追加する。
- フロントエンドの fallback 表示は開発用モードで常に有効。設定で無効化するトグルは `docs/design/tauri_gui_minimal_flow.md` の F4 タスクで扱う。
- IME 入力、ファイルダイアログの自動テスト、E2E（Playwright 等）の導入は後続タスクで検討する。
- `cargo test -p altre-tauri-app` はネイティブ依存が不足している環境では失敗するため、Nix シェルや個別パッケージの導入が必須。
