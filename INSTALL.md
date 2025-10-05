# altre インストールガイド

## 動作確認済み環境
- Linux (x86_64): NixOS / Ubuntu 22.04 / Arch Linux
- macOS 14 (Apple Silicon): 開発者ローカルでの動作確認
- Windows 11 (WSL2): raw mode 対応端末で最小限の確認

## 1. 取得とビルド
### 依存パッケージ
- Rust 1.78 以上（`rustup` 推奨）
- C コンパイラ（clang または gcc）
- UTF-8 表示に対応した端末

### ソース取得とビルド
```bash
git clone <このリポジトリのURL>
cd altre
cargo build --release --offline   # ネットワークが使える場合は --offline を省略可
```
生成物は `target/release/altre` に出力されます。

### テストとドキュメント
```bash
cargo test --offline
cargo doc --no-deps --offline
```

## 2. 実行
```bash
cd altre
cargo run --release --offline
```
raw mode を利用するため、端末や仮想環境によっては実行に失敗することがあります。その場合は `manuals/troubleshooting.md` を参照してください。

## 3. 設定
MVP 版にはユーザー設定ファイルはありません。将来的に `~/.altre.d/` 配下へ設定ファイルを配置する予定です。

## 4. よくある問題
- Alt キーが Meta として認識されない: `Esc` をプレフィックスとして使用（例: `Esc x`）
- 描画が乱れる: 端末サイズを固定するか `cargo run --release --offline` を使用
- raw mode が拒否される: 別の端末エミュレータを使用するか TUI 実行を避けてテスト/ビルドのみ行う

## 5. アップデート
公式バイナリ配布は未整備です。更新が必要な場合はローカルで再ビルドしてください。詳細な運用メモは `manuals/` 配下を参照します。
