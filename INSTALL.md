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
  - **NixOS**: `slint`, `wayland`, `wayland-protocols`, `libxkbcommon`, `fontconfig`, `freetype`, `harfbuzz`, `mesa`, `libGL`, `vulkan-loader`, `pkg-config`
  - **Ubuntu / Debian / Linux Mint**:
    ```
    sudo apt install libwayland-dev wayland-protocols libxkbcommon-dev \
      libfreetype6-dev libfontconfig1-dev libharfbuzz-dev \
      libgl1-mesa-dev libegl1-mesa-dev vulkan-loader pkg-config
    ```
  - **Fedora**:
    ```
    sudo dnf install wayland-devel wayland-protocols-devel libxkbcommon-devel \
      libX11-devel libXext-devel libXcursor-devel libXi-devel \
      freetype-devel fontconfig-devel harfbuzz-devel \
      mesa-libGL-devel mesa-libEGL-devel vulkan-loader pkgconf-pkg-config
    ```
  - **Windows**: Visual Studio Build Tools、CMake、`winget install slint.slint`（または Cargo 経由）、GPU ドライバ
  - **macOS**: Xcode Command Line Tools、`brew install slint freetype harfbuzz fontconfig pkg-config`
  - 依存の背景と詳細は `docs/design/slint_dependency_minimum.md` を参照

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
3. `cargo tauri dev --manifest-path src-tauri/Cargo.toml` を実行するとウィンドウが起動する（現状は fallback UI 表示）。
   - ネットワークアクセスが必須。`tauri` クレートや npm 依存を取得できない場合は、後でリトライする。
4. ブラウザで確認するだけなら `npm run dev --prefix frontend/react` を利用し、`http://localhost:5173` を開く。

## 3. 設定
MVP 版にはユーザー設定ファイルはありません。将来的に `~/.altre.d/` 配下へ設定ファイルを配置する予定です。

## 4. よくある問題
- Alt キーが Meta として認識されない: `Esc` をプレフィックスとして使用（例: `Esc x`）
- 描画が乱れる: 端末サイズを固定するか `cargo run --release --offline` を使用
- raw mode が拒否される: 別の端末エミュレータを使用するか TUI 実行を避けてテスト/ビルドのみ行う

## 5. アップデート
公式バイナリ配布は未整備です。更新が必要な場合はローカルで再ビルドしてください。詳細な運用メモは `manuals/` 配下を参照します。
