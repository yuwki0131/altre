# デバッグモード

## 概要
altreエディタはオプションのデバッグモードを提供しています。デバッグモードでは、ファイル操作などの内部動作に関する詳細なログが出力されます。

## 使用方法

### デバッグモードを有効にする
```bash
# 環境変数を設定してデバッグモードで実行
ALTRE_DEBUG=1 cargo run

# または
export ALTRE_DEBUG=1
cargo run
```

### 通常モード（デバッグ出力なし）
```bash
# 環境変数なしで実行
cargo run
```

## デバッグ出力の内容

### ファイル操作
- ファイルのオープン処理
- ファイル保存処理（パス、内容サイズ、アトミック保存の詳細）
- ファイル存在確認
- エラー詳細

### 出力例
```
DEBUG: Opening file: /path/to/file.txt
DEBUG: File opened successfully, editor synchronized
DEBUG: Saving to path: /path/to/file.txt
DEBUG: Content length: 123 chars
DEBUG FileSaver: save_file called with path: /path/to/file.txt
DEBUG FileSaver: using atomic save
DEBUG FileSaver: atomic_save: temp_path: /path/to/.file.txt_12345
DEBUG FileSaver: atomic_save: rename completed successfully
DEBUG: FileSaver reported success
DEBUG: File exists after save
```

## 実装詳細
- 環境変数 `ALTRE_DEBUG` の存在でデバッグモードを判定
- `debug_log!` マクロによる条件付きログ出力
- ファイル操作専用の `file_debug_log!` マクロ