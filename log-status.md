# ログステータス（2025-10-20）

## 依存関係の状況
- Rust 側は `tauri 2.8.5` / `tauri-runtime-wry 2.8.0` / `softbuffer 0.4.6` を使用。`softbuffer` が `objc2 0.5.2` を要求するため、`cargo tree -i objc2@0.5.2` にはまだ参照が残る。
- GUI ビルドには GTK / WebKitGTK / libsoup / libappindicator などのネイティブ依存が必須。Nix 環境では `nix/shell.nix` でセットアップ可能。
- Node.js 18 以上と `@tauri-apps/cli`、`npm install --prefix frontend/react` の依存が揃っていればホットリロードまで含めて動作する。

## ビルド確認結果
- `cargo check -p altre` / `cargo check -p altre-tauri` / `cargo check -p altre-tauri-app` は Nix シェル経由で成功。
- `cargo run -p altre -- --gui` は初回に `altre-tauri-app` を自動ビルドし、GUI バイナリを起動。GUI バイナリが起動できない場合は TUI へフォールバックする。
- `cargo run -p altre -- --tui` は従来通り ratatui の TUI を起動。raw mode が利用できない端末では失敗するため、テストのみ行う場合は `cargo test`/`cargo check` を利用する。
- `npm --prefix frontend/react run build` が通った後で `cargo tauri dev --manifest-path src-tauri/Cargo.toml` を実行するとホットリロード付きで GUI を確認できる。

## 今後の対応案
- `docs/design/tauri_gui_minimal_flow.md` の T3/F3/F4/QA1（スナップショット拡張、fallback の切り替え、E2E 手順）は継続課題。
- `cargo test -p altre-tauri` / `-p altre-tauri-app` をオフラインでも通せるよう、依存キャッシュおよび CI 相当の実行手順を整備する。
- `softbuffer` の `objc2 0.6` 対応版が公開されたら更新し、依存木から 0.5 系を排除する。

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
- `altre` バイナリに `--tui` / `--gui` オプションを追加し、デフォルトで GUI 起動、失敗時は TUI へフォールバックする挙動を実装。GUI バイナリは自動的に `cargo build -p altre-tauri-app --release` で生成し、`target/release/altre-tauri-app` を起動。`cargo check -p altre` で確認済み（2025-03-16）。
- GUI 経由でスペースキーを押すとエラーになっていた問題を修正（`altre-tauri/src/keymap.rs`）。スペースを `trim()` せず扱い、単体テストを追加。`npm --prefix frontend/react run build` で確認（2025-03-16）。
- GUI ヘッダーに `開く…` / `保存` ボタンを追加し、`保存` 成功時はレスポンスメッセージをミニバッファへ表示。`開く…` は Tauri ダイアログ未提供環境では `window.prompt` を用いたパス入力へフォールバック（2025-03-16）。

## 今後の対応案
- `frontend/react` の npm 依存を整えて `npm --prefix frontend/react run build` を安定化。その上で `cargo tauri dev` でのランタイム確認を行う。
- `softbuffer` が `objc2 0.6` 対応版を公開したら `cargo update` を再実行し、`cargo tree -i objc2@0.5.2` が空になることを確認する。必要に応じてローカルパッチや vendoring で `softbuffer` の依存を更新することも検討する。
