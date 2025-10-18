# Slint依存ライブラリ最小構成調査

## 概要
Slint GUI をビルド・実行するために必要なランタイム／ライブラリを調査し、ターゲット OS ごとの最小構成を整理する。

## 共通要件
- Rust 1.78 以降
- Slint クレート（`slint`）
- GPU/ソフトウェアレンダリング用バックエンド（OpenGL/EGL/Vulkan のいずれか）
- フォント・テキストレンダリングライブラリ（FreeType, Fontconfig, HarfBuzz）
- Wayland/X11 用のクライアントライブラリ（Linux のみ）
- `pkg-config`（ビルド時のライブラリ検出に使用）

### Cargo 設定
- `Cargo.toml` では `slint` をオプション依存として追加し、`gui` フィーチャーに紐付ける。
- `slint::include_slint!` を利用する予定のため `slint-build` は現状不要。

## NixOS
| 目的 | パッケージ |
|------|------------|
| Slint ランタイム | `slint`（クレート）、`qt6.qtwayland`（必要に応じて） |
| フォント | `freetype`, `fontconfig`, `harfbuzz` |
| Wayland | `wayland`, `wayland-protocols`, `libxkbcommon` |
| X11 互換 | `libX11`, `libXext`, `libXcursor`, `libXi` |
| GPU | `mesa`, `libGL`, `vulkan-loader`, `egl-wayland` |
| ツール | `pkg-config` |

例: shell.nix への追加
```nix
buildInputs = [
  pkgs.slint
  pkgs.wayland
  pkgs.wayland-protocols
  pkgs.libxkbcommon
  pkgs.freetype
  pkgs.fontconfig
  pkgs.harfbuzz
  pkgs.libGL
  pkgs.mesa
  pkgs.vulkan-loader
  pkgs.pkg-config
];
```

## Ubuntu / Debian / Linux Mint
```
sudo apt install \
  libslint-dev (無い場合は cargo で取得) \
  libwayland-dev wayland-protocols libxkbcommon-dev \
  libx11-dev libxext-dev libxcursor-dev libxi-dev \
  libfreetype6-dev libfontconfig1-dev libharfbuzz-dev \
  libgl1-mesa-dev libegl1-mesa-dev vulkan-loader \
  pkg-config
```
- Ubuntu 22.04 以降を想定。Wayland/Hyprland の場合は `libwayland-dev` が必須。
- GPU が無い環境では `mesa` のソフトウェアレンダリングが利用される。

## Fedora
```
sudo dnf install \
  wayland-devel wayland-protocols-devel libxkbcommon-devel \
  libX11-devel libXext-devel libXcursor-devel libXi-devel \
  freetype-devel fontconfig-devel harfbuzz-devel \
  mesa-libGL-devel mesa-libEGL-devel vulkan-loader \
  pkgconf-pkg-config
```

## Windows
- 必須: Visual Studio Build Tools (MSVC), CMake, Ninja（Slint 推奨）、`cargo`。
- 依存ライブラリは Slint バンドルに含まれるが、GPU ドライバ（DirectX 12 / Vulkan）が必要。
- 追加の DLL は不要だが、ランタイム配布時は `slint.dll` などの再配布を確認。
- 手順:
  1. `winget install -e --id Python.Python.3.12`（build.rs で python が必要な場合）
  2. `winget install -e --id Kitware.CMake`
  3. `winget install -e --id Microsoft.VisualStudio.2022.BuildTools`

## macOS
- Xcode Command Line Tools (`xcode-select --install`)
- Homebrew パッケージ:
  ```
  brew install slint
  brew install freetype harfbuzz fontconfig
  brew install pkg-config
  ```
- OpenGL ドライバは macOS 標準の Metal ラッパーが利用される。Vulkan を使う場合は `brew install molten-vk` が必要（任意）。

## 追加メモ
- `slint::include_slint!` を用いるため、`build.rs` は不要だが `.slint` ファイルを配置するディレクトリを Cargo に含める必要がある。
- CI に導入する際は、Nix ベースの場合 `nativeBuildInputs` に上記パッケージを追加。
- 依存ライセンス:
  - FreeType (FTL), Fontconfig (MIT), HarfBuzz (MIT), Wayland (MIT), libxkbcommon (MIT), Mesa (MIT), Vulkan-loader (Apache 2.0)
  - 再配布時はそれぞれのライセンス文書を `dist/` に同梱すること。

## 今後のタスク
- Nix shell への反映（`tasks/todo/future/slint_dependency_setup_implementation.md`）
- README/manuals への依存インストールガイド追記（GUI ドキュメント更新タスクで対応）
