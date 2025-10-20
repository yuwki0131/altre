# ログステータス（2025-03-16）

## 依存関係の状況
- `cargo update -p tauri-runtime-wry` / `softbuffer` / `tauri` を実行したが、いずれも最新互換版が取得済みでロックファイルに更新はなし。
- `softbuffer 0.4.6` が `objc2 0.5.2` を要求しており、これが `cargo tree -i objc2@0.5.2` に残る要因。現時点で `softbuffer` の `objc2 0.6` 対応版は crates.io に存在しない。
- `tauri-runtime` は最新が 2.8.0 で固定されており、`tauri 2.8.5` との整合性は保たれている。
- `nix/shell.nix` に GTK / WebKit / libsoup / libappindicator など Linux GUI 依存を追加し、`PKG_CONFIG_PATH` と `LD_LIBRARY_PATH` をシェル起動時にセットするよう更新（2025-03-16）。

## ビルド確認結果
- `cargo check -p altre` は成功し、TUI クレートは問題なし。
- `cargo check -p altre-tauri-app` は `atk/gdk/gobject/pango` など GTK 系システムライブラリを `pkg-config` が検出できず失敗。GUI ビルドにはネイティブ依存（.pc ファイル）の導入が必要。
- `nix-shell nix/shell.nix --command 'cargo check -p altre-tauri-app'` が成功。`src-tauri/icons/icon.png` を RGBA 32bit PNG へ差し替えて Tauri のアイコン検証を通過（2025-03-16）。

## 今後の対応案
- `frontend/react` の npm 依存を整えて `npm --prefix frontend/react run build` を安定化。その上で `cargo tauri dev` でのランタイム確認を行う。
- `softbuffer` が `objc2 0.6` 対応版を公開したら `cargo update` を再実行し、`cargo tree -i objc2@0.5.2` が空になることを確認する。必要に応じてローカルパッチや vendoring で `softbuffer` の依存を更新することも検討する。
