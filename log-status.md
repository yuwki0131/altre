# ログステータス（2025-03-16）

## 依存関係の状況
- `cargo update -p tauri-runtime-wry` / `softbuffer` / `tauri` を実行したが、いずれも最新互換版が取得済みでロックファイルに更新はなし。
- `softbuffer 0.4.6` が `objc2 0.5.2` を要求しており、これが `cargo tree -i objc2@0.5.2` に残る要因。現時点で `softbuffer` の `objc2 0.6` 対応版は crates.io に存在しない。
- `tauri-runtime` は最新が 2.8.0 で固定されており、`tauri 2.8.5` との整合性は保たれている。
- `nix/shell.nix` に GTK / WebKit / libsoup / libappindicator など Linux GUI 依存を追加し、`PKG_CONFIG_PATH` と `LD_LIBRARY_PATH` をシェル起動時にセットするよう更新（2025-03-16）。

## ビルド確認結果
- `cargo check -p altre` は成功し、TUI クレートは問題なし。
- ホスト環境での `cargo check -p altre-tauri-app` は引き続き GTK 系ネイティブ依存が不足して失敗するため、`nix-shell` 利用が必須。
- `nix-shell nix/shell.nix --command 'cargo check -p altre-tauri-app'` が成功。`src-tauri/icons/icon.png` を RGBA 32bit PNG へ差し替えて Tauri のアイコン検証を通過（2025-03-16）。
- `editor_init` コマンドを追加し、GUI 起動時にログ出力先・初期ファイル・ワーキングディレクトリを指定できるよう更新。`cargo check -p altre-tauri` / `nix-shell nix/shell.nix --command 'cargo check -p altre-tauri-app'` で動作確認済み（2025-03-16）。
- `KeySequencePayload` をチャンク形式（`Vec<Vec<KeyStrokePayload>>`）へ拡張し、React 側はタイムアウト付きバッファで複合キーをまとめて送信可能にした。`cargo check -p altre-tauri` と `npm --prefix frontend/react run build` を実施済み（2025-03-16）。
- Tauri 実行時はフォールバックを無効化し、`invoke` 失敗時のエラーメッセージを React で表示できるよう調整。ブラウザプレビュー時は従来のフォールバックを維持（2025-03-16）。

## 今後の対応案
- `frontend/react` の npm 依存を整えて `npm --prefix frontend/react run build` を安定化。その上で `cargo tauri dev` でのランタイム確認を行う。
- `softbuffer` が `objc2 0.6` 対応版を公開したら `cargo update` を再実行し、`cargo tree -i objc2@0.5.2` が空になることを確認する。必要に応じてローカルパッチや vendoring で `softbuffer` の依存を更新することも検討する。
