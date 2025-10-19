# Tauri プロジェクト構成整備

## タスク概要
Tauri GUI を追加開発できるよう、モノレポ構成・ワークスペース設定・フロントエンド雛形を整備する。

## 目的
- Rust/Tauri/React の各レイヤをビルドできるプロジェクト構成を用意する。
- NixOS 環境での依存インストールとビルド手順を確認する。
- GUI なしでも既存 TUI がビルド・実行できることを保証する。

## 成果物
- `frontend/react/`（仮）に React 雛形を生成し、`package.json`・ロックファイルを整備。
- `src-tauri/`（別名採用）に Tauri 設定ファイルとエントリポイントを配置。
- `Cargo.toml` をワークスペース構成に更新し、TUI クレートとのビルド連携を確認。
- NixOS での `npm install` / `tauri dev` 手順メモと既知の制限事項。

## 前提条件
- ADR 0007 および Tauri アーキテクチャ設計タスクの成果物が参照可能であること。
- React を採用する方針（QA Q29）が確定済みであること。

## 完了条件
- [ ] ルートで `cargo check` が通り、TUI 単体動作に影響がない。
- [ ] `npm install`（または `pnpm install`）が NixOS 上で成功する手順が記録されている。
- [ ] `tauri dev`（GUI 起動）が最低限のウィンドウ表示まで確認できる。
- [ ] README / INSTALL に必要最小限のセットアップ追記項目が列挙されている。

## 進捗メモ
- 2025-03-15: ワークスペース化 (`altre-core` / `altre-tauri`) と React 雛形 (`frontend/react`) を追加。`cargo check` は成功済み。
- 2025-03-15: README に TUI/GUI の想定コマンドと NixOS 向け `npm install` 手順メモを追記。npm 実行確認と `tauri dev` は未対応。

## メモ
- NixOS で標準 npm コマンドが失敗する場合は `nix develop` などの回避策を記録する（QA Q34）。
- 既存 `src/main.rs` の起動判定は変更せず、GUI 起動は Tauri コマンド側で提供する。

## 見積もり
**期間**: 3日  
**優先度**: 高

## 関連タスク
- Tauri GUI アーキテクチャ詳細設計
- Tauri バックエンド連携実装
- Tauri GUI ドキュメント更新
