# altre インストールガイド

## 動作確認済み環境
- Linux (x86_64) — Ubuntu 22.04 / Arch Linux
- macOS 14 (Apple Silicon) — 開発者テスト
- Windows 11 (WSL2) — raw mode 対応端末で動作

## 1. バイナリからのインストール
1. `dist/altre-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` を取得
2. 展開
   ```bash
   tar -xzf altre-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
   sudo install -m 755 altre /usr/local/bin/altre
   ```
3. 動作確認
   ```bash
   altre --version
   ```
4. アンインストールは `/usr/local/bin/altre` を削除

### チェックサム検証
```bash
sha256sum -c altre-v0.1.0-x86_64-unknown-linux-gnu.sha256
```

## 2. ソースからのビルド
### 依存パッケージ
- Rust 1.74 以上 (`rustup` 推奨)
- C コンパイラ (clang または gcc)
- UTF-8 対応ターミナル

### 手順
```bash
git clone https://github.com/altre-editor/altre.git
cd altre/app
cargo build --release
```
- 成功すると `app/target/release/altre` が生成されます。

### テスト
```bash
cargo test
cargo doc --no-deps
```

## 3. 初回起動
```bash
cargo run --release
```
- raw mode を利用するため、失敗した場合は `manuals/troubleshooting.md` を参照

## 4. 設定ファイル
- MVP 版にはユーザー設定ファイルはありません。将来 `~/.altre.d/` 配下に追加予定です。

## 5. よくある問題
- Alt キーが Meta として認識されない → `Esc x` のように Esc をプレフィックスとして使用
- 画面描画が崩れる → 端末サイズを固定するか、`cargo run --release` を利用

## 6. アップデート
- リリースタグ `v0.1.0` 以降、`CHANGELOG.md` を参照して差分を確認してください。

ご不明な点があれば `manuals/faq.md` または GitHub Issue で質問してください。
