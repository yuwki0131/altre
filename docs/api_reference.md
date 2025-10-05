# altre API リファレンス（MVP）

本ドキュメントはソースコードの主要構造体・関数の概要を日本語で整理したものです。詳細なシグネチャは Rustdoc (`cargo doc --open`) で確認してください。

## 1. エントリポイント
### `App` (`src/app.rs`)
- **役割**: イベントループと全体状態の管理
- **主要メソッド**
  - `App::new()` : アプリケーション状態を初期化
  - `App::run()` : メインループを開始
  - `App::handle_command(Command)` : 解決済みコマンドを実行
  - `start_find_file_prompt` / `start_save_as_prompt` : ミニバッファを初期化

### 使用例
```rust
let mut app = App::new()?;
app.run()?;
```

## 2. 入力とコマンド
### `Command` (`src/input/commands.rs`)
- Emacs 風の編集／ファイル操作を列挙体で表現
- `Command::execute` ではなく、`CommandProcessor` が実体を処理

### `CommandProcessor`
- **責務**: `TextEditor` / `FileOperationManager` / ミニバッファとの橋渡し
- **主要メソッド**
  - `execute(Command) -> CommandResult`
  - `open_file(path: String) -> CommandResult`
  - `save_buffer()` / `save_buffer_as(path: String)`
- **エラー処理**: `CommandResult::error(message)` でメッセージを返す

## 3. バッファ管理
### `TextEditor` (`src/buffer/editor.rs`)
- ギャップバッファを利用したテキスト編集
- 主な API
  - `insert_char(char)`
  - `delete_backward()` / `delete_forward()`
  - `navigate(NavigationAction)`
  - `to_string()` で現在内容を取得

### `FileBuffer` (`src/file/operations.rs`)
- ディスク上のファイルとエディタ内容を同期
- 新設 `save_as(PathBuf)` は別名保存を実行し、`change_tracker` を更新

## 4. ミニバッファ
### `MinibufferSystem` (`src/minibuffer/system.rs`)
- **機能**: キー入力→`SystemResponse` 変換、コマンド実行結果の橋渡し
- **関数**
  - `handle_event(SystemEvent) -> Result<SystemResponse>`
  - `start_find_file` / `start_execute_command` / `start_write_file`
  - `show_error(String)` / `show_info(String)`

### `SystemResponse`
- `FileOperation(FileOperation)` / `ExecuteCommand(String)` / `Quit` などアプリ側への通知

## 5. ファイル I/O
### `FileOperationManager`
- `open_file(PathBuf) -> Result<FileBuffer>`
- `save_buffer(&mut FileBuffer)`
- `save_buffer_as(&mut FileBuffer, PathBuf)`
- ファイル保存時は `NewFileHandler::handle_new_file` でディレクトリと権限を検証

## 6. alisp
### `Interpreter` (`src/alisp/`)
- `eval_str(&mut self, expr: &str) -> EvalOutcome`
- `register_builtin(name, function)` でビルトイン登録（詳細は `docs/design/alisp_language_spec.md`）

## 7. エラーとロギング
- `crate::error::AltreError` がアプリ全体の共通エラー型
- `CommandResult`、`MinibufferResult` などはユーザー向けメッセージを含む
- `debug_log!(app, ...)` マクロでデバッグログを出力（`RUST_LOG` と連携予定）

## 8. サンプルワークフロー
```rust
let mut processor = CommandProcessor::new();
let open = processor.open_file("README.md".to_string());
if open.success {
    processor.execute(Command::InsertChar('A'));
    processor.execute(Command::SaveBuffer);
}
```

## 9. パフォーマンスに関する注意
- ギャップバッファ操作は O(1) を目指していますが、巨大バッファでは再配置が発生します。
- `save_buffer` は変更がない場合にディスク書き込みをスキップします。
- ミニバッファ補完は最大 50 件まで返し、`PathCompletion` がキャッシュを持ちます。

## 10. テスト
- 単体テスト: `#[cfg(test)]` ブロックでモジュール内部の API を検証
- 結合テスト: `tests/` に配置、`cargo test` で実行
- ベンチマーク: `benches/`、`cargo bench --offline` を想定

## 11. ドキュメント生成
- `scripts/generate_docs.sh` を利用して `cargo doc --no-deps` を実行
- 生成物は `target/doc/altre/index.html`
- CI 連携時は `cargo doc --no-deps --document-private-items` を追加し、警告ゼロを目標とする

---
API の詳細と最新状態は常にソースコードを参照してください。コントリビューション時は Rustdoc コメントの更新も忘れずに行ってください。
