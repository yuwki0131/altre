# Slint依存ライブラリ導入

## タスク概要
調査結果に基づき、Slint GUI をビルド・実行するための追加依存ライブラリをプロジェクト環境（NixOS・他 OS）のセットアップ手順に組み込む。

## 目的
- 開発環境および将来の CI で Slint GUI を問題なくビルドできるようにする
- `Cargo.toml` や Nix 設定、補助スクリプトを更新して開発者が迷わず環境構築できるようにする
- 依存追加に伴うライセンスやバイナリサイズの影響を把握する

## 実装範囲
1. Nix shell / derivation 更新（必要パッケージ追加）
2. 他 OS 向けセットアップガイドへの追記
3. `Cargo.toml`、`build.rs` 不要確認、`slint::include_slint!` 利用のための設定
4. 依存導入後のビルド確認とドキュメント反映

## 前提条件
- Slint 依存ライブラリ最小構成調査タスクの完了
- Slint GUI フロントエンド実装の着手前後で実行

## 完了条件
- [x] NixOS で GUI ビルド・実行が成功（`nix/shell.nix` を追加）
- [x] Ubuntu/Debian 等で必要パッケージがガイドに記載されている
- [x] Windows/macOS のセットアップ手順が更新
- [x] 変更内容がドキュメントおよび CHANGELOG に反映

## 実施メモ
- `Cargo.toml` にオプション依存 `slint` を追加し、`gui` フィーチャーと連携。
- `nix/shell.nix` を新規作成し、Slint・Wayland・フォント関連パッケージを同梱。
- `INSTALL.md` に各 OS 向けの追加パッケージ一覧と Nix シェル利用方法を追記。
- `README.md` のセットアップ節に GUI 依存の概要を追加し、`CHANGELOG.md` に記録。

## 見積もり
**期間**: 1日
**優先度**: 中（GUI 実装準備）

## 関連タスク
- Slint 依存ライブラリ最小構成調査
- Slint GUI フロントエンド実装
