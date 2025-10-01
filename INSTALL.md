# altre インストールガイド

## 動作確認済み環境
- Linux (x86_64) — Ubuntu 22.04 / Arch Linux
- macOS 14 (Apple Silicon) — 開発者テスト
- Windows 11 (WSL2) — raw mode 対応端末で動作

## 1. ソースからのビルド（開発者向けメモ）
### 依存パッケージ
- Rust 1.74 以上 (`rustup` 推奨)
- C コンパイラ (clang または gcc)
- UTF-8 対応ターミナル

### 手順
```bash
git clone <このリポジトリのローカルURL>
cd altre/app
cargo build --release
```
- 成功すると `app/target/release/altre` が生成されます。

### テスト
```bash
cargo test
cargo doc --no-deps
```

## 2. 初回起動
```bash
cargo run --release
```
- raw mode を利用するため、失敗した場合は `manuals/troubleshooting.md` を参照

## 3. 設定ファイル
- MVP 版にはユーザー設定ファイルはありません。将来 `~/.altre.d/` 配下に追加予定です。

## 4. よくある問題
- Alt キーが Meta として認識されない → `Esc x` のように Esc をプレフィックスとして使用
- 画面描画が崩れる → 端末サイズを固定するか、`cargo run --release` を利用

## 5. アップデート
- バイナリ配布や公式リリース手順は未整備です。必要に応じてローカル環境でビルドを行ってください。

疑問点やトラブルは、自分用メモである `manuals/faq.md` や `manuals/troubleshooting.md` を参照してください。
