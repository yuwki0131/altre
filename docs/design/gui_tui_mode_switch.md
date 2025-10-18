# GUI/TUIモード切替設計

## 概要
altre は GUI（Slint）と TUI（ratatui）を同一バイナリで提供する。デフォルトで GUI を起動し、コマンドラインオプションで TUI を選択できるようにするための設計方針を定義する。

## 要件
- `altre` コマンドを実行した際に、GUI モードをデフォルト起動とする。
- `--tui` オプションで TUI モードを明示的に指定できる。
- `--gui` オプションを提供し、将来的なデフォルト変更やスクリプト用に明示指定可能にする。
- GUI 起動時に必要な依存が不足している場合、自動で TUI へフォールバックするか、明確なエラーを表示する。
- 起動オプションは `--help` に表示し、README/manuals に記載する。

## CLI インターフェース
```
altre [OPTIONS]

Options:
    --gui             GUI モードで起動（デフォルト）
    --tui             TUI モードで起動
    --headless        将来用（テスト向け、現在は未実装）
    --config <PATH>   設定ファイル指定（GUI/TUI 共通、将来拡張）
    -h, --help        ヘルプ表示
    -V, --version     バージョン表示
```

優先順位:
1. `--gui` / `--tui` が指定されていればそれを採用
2. 指定がなければ GUI を選択
3. GUI 起動失敗時は `--tui` が指定されていない限り、TUI へフォールバックするか、エラーのまま終了するかを選択できるようにする（実装タスクでフラグを検討）

## 起動シーケンス
```
main.rs
  ├─ parse_cli_options()
  ├─ initialize_logger()
  ├─ match mode {
  │     Mode::Gui => run_gui()
  │     Mode::Tui => run_tui()
  │ }
```

### run_gui()
1. Slint 依存を初期化（必要なら環境チェック）
2. CoreBackend::new() を呼び出しアプリケーション状態を構築
3. `frontend::gui::run(backend, cli_options)` を実行
4. エラー時は `GuiLaunchError` を返却、フォールバック条件を判定

### run_tui()
1. 端末の raw mode と alternate screen を設定
2. CoreBackend::new() を呼び出し
3. `frontend::tui::run(backend, cli_options)` を実行
4. 終了時に端末を復旧

## フォールバック方針
- GUI 起動に失敗した場合のパターン:
  1. Slint runtime 初期化失敗（依存不足）
  2. ウィンドウ作成失敗（ヘッドレス環境など）
  3. GPU / OpenGL バックエンドエラー
- 初期実装では、エラー内容を表示したうえで終了する。フォールバック自動化は将来の挙動として CLI フラグで切り替える想定（例: `--gui-fallback=tui`）。

## エラーメッセージ
- GUI 依存不足例:
  ```
  GUI モードの初期化に失敗しました: Missing library: libwayland-client.so
  `altre --tui` で TUI モードを利用するか、依存パッケージをインストールしてください。
  ```
- TUI raw mode 失敗例:
  ```
  TUI モードを起動できませんでした: Failed to enter raw mode
  端末が raw mode をサポートしていない可能性があります。`altre --gui` を試してください。
  ```

## 設定ファイルとの関係
- 将来的に設定ファイル（alisp）からモードを指定できるようにする場合、CLI オプションが優先度最上位とする。
- 設定ファイルによるモード指定は未実装。設計段階では CLI オプションのみに集中する。

## ドキュメント更新ポイント
- README「実行方法」節に CLI オプションとフォールバック行動を追記。
- manuals/user_guide.md に GUI/TUI 起動例を追加。
- troubleshooting に依存不足時の対処を記載。

## 今後の拡張
- `--headless`（テスト用ダミーフロントエンド）の実装
- GUI → TUI 自動フォールバックの条件設定（環境変数や設定ファイルによる制御）
- Windows/macOS 向けのショートカット生成・バンドル対応
