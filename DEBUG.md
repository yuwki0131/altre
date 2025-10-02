# デバッグモード

## 概要
altre エディタではデバッグ向けの詳細ログを環境変数で有効化できます。ファイル操作やミニバッファ経由のコマンド実行など、開発時に把握したい情報を標準エラー出力へ表示します。

## デバッグモードの有効化
```bash
cd altre/app
ALTRE_DEBUG=1 cargo run --offline
```
または、環境変数を先に設定してから `cargo run` / `cargo test` を実行します。

```bash
cd altre/app
export ALTRE_DEBUG=1
cargo run --offline
```

## 通常モード（デバッグ出力なし）
```bash
cd altre/app
cargo run --offline
```

## 出力内容の例
```
DEBUG: Opening file: /path/to/file.txt
DEBUG: File opened successfully, editor synchronized
DEBUG FileSaver: save_file called with path: /path/to/file.txt
DEBUG FileSaver: using atomic save
DEBUG FileSaver: atomic_save: temp_path: /path/to/.file.txt_12345
DEBUG FileSaver: atomic_save: rename completed successfully
```

## 実装メモ
- `ALTRE_DEBUG` の有無でデバッグモードを判定
- `debug_log!`（`app/src/app.rs`）と `file_debug_log!`（`app/src/file/operations.rs`）で条件付きログを出力
- ファイル操作、ミニバッファコマンド、イベント処理の要所でログを仕込み済み
