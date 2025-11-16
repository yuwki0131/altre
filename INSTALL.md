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
- GUI 開発時は追加で以下をインストール
  - Node.js 18 以上と npm、`npm install -g @tauri-apps/cli`
  - **NixOS**: `nix-shell nix/shell.nix` で GTK / libsoup / WebKitGTK などを含む環境を提供
  - **Ubuntu / Debian / Linux Mint**:
    ```
    sudo apt install build-essential pkg-config libgtk-3-dev \
      libayatana-appindicator3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev \
      libssl-dev libxkbcommon-dev libfontconfig1-dev libfreetype6-dev \
      libharfbuzz-dev libglu1-mesa-dev libegl1-mesa
    ```
  - **Fedora**:
    ```
    sudo dnf install gcc-c++ pkgconf-pkg-config gtk3-devel \
      libappindicator-gtk3-devel libsoup3-devel webkit2gtk4.1-devel \
      freetype-devel fontconfig-devel harfbuzz-devel mesa-libGL-devel
    ```
  - **Windows**: Visual Studio Build Tools、CMake、`winget install NodeJS.LTS`、`npm install -g @tauri-apps/cli`
  - **macOS**: Xcode Command Line Tools、`brew install node tauri-cli gtk+3 libsoup webkit2gtk`

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

## 3. NixOS での開発シェル
`nix/shell.nix` を用意しています。Nix 環境では以下で必要パッケージが揃ったシェルに入ります。
```bash
nix-shell
```


## GUI (Tauri) のビルド / 実行
1. `nix-shell nix/shell.nix` で開発シェルに入る（Node.js と Tauri CLI を含む環境を想定）。
2. `npm install --prefix frontend/react` で依存を取得し、`npm run build --prefix frontend/react` で `dist/` を生成。
3. `cargo run -p altre -- --gui` で GUI を起動する。初回は `target/release/altre-tauri-app` を自動ビルドし、GUI バイナリが起動する。
   - `cargo run -p altre`（オプションなし）でも GUI を優先して起動し、失敗時に TUI へフォールバックする。
   - `cargo tauri dev --manifest-path src-tauri/Cargo.toml` を実行するとホットリロード可能な開発モードで GUI を起動できる（ネットワークアクセス必須）。
4. ブラウザで見た目のみ確認する場合は `npm run dev --prefix frontend/react` を利用し、`http://localhost:5173` を開く（この場合はバックエンド未接続の fallback 状態）。
5. GUI 実行時のログは `ALTRE_GUI_DEBUG_LOG` 環境変数で出力先を指定でき、既定では `~/.altre-log/debug.log` に JSON Lines 形式で記録される。

## 3. 設定
MVP 版にはユーザー設定ファイルはありません。将来的に `~/.altre.d/` 配下へ設定ファイルを配置する予定です。

## 4. よくある問題
- Alt キーが Meta として認識されない: `Esc` をプレフィックスとして使用（例: `Esc x`）
- 描画が乱れる: 端末サイズを固定するか `cargo run --release --offline` を使用
- raw mode が拒否される: 別の端末エミュレータを使用するか TUI 実行を避けてテスト/ビルドのみ行う

## 5. アップデート
公式バイナリ配布は未整備です。更新が必要な場合はローカルで再ビルドしてください。詳細な運用メモは `manuals/` 配下を参照します。
