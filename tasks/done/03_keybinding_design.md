# キーバインド設計

## タスク概要
MVPで実装するキーバインドシステムの設計を行う。

## 目的
- Emacs風キーバインドの効率的な実装
- 連続キー（C-x C-f等）の適切な処理
- 将来のキーバインドカスタマイズ機能への拡張性確保

## 設計対象
1. **キーバインド表現**
   - キー入力の内部表現
   - 修飾キー（Ctrl、Alt等）の処理
   - 連続キーシーケンスの状態管理

2. **MVP対象キーバインド**
   - `C-x C-f`: ファイルオープン
   - `C-x C-s`: ファイル保存
   - `C-x C-c`: 終了
   - `C-n`/`C-p`/`C-f`/`C-b`: カーソル移動
   - 矢印キー: カーソル移動
   - `Backspace`/`Delete`: 文字削除
   - `Enter`: 改行
   - `M-x`: コマンド実行

3. **キーバインド処理アーキテクチャ**
   - キー入力からアクション実行までのフロー
   - 部分マッチ状態の管理
   - タイムアウト処理

4. **crossterm統合**
   - ターミナル固有のキーコード差異の吸収
   - 特殊キーの正規化

## 成果物
- `docs/design/keybinding.md` - 設計仕様書
- `src/input/keybinding.rs` - インターフェース定義
- キーバインド定義ファイル仕様

## QA要確認事項
- キーマップ階層（MVP では単一階層でOK？）
- OS/IME衝突回避の実装範囲
- 不正キー入力時の動作（「無視」の詳細仕様）

## 完了条件
- [x] キーバインド表現方法の確定（`docs/design/keybinding.md:12` と `src/input/keybinding.rs:1` にて定義済み）
- [x] MVP対象キーの完全リスト化（`manuals/keybinding_reference.md:1` に一覧化）
- [x] 処理フロー設計の完成（`docs/design/keybinding.md:163` に入力→コマンドフローを記載）
- [x] エラーケース処理方針の策定（`src/input/event_handler.rs:1` で未定義キーの扱いを実装）

## ステータス
- `tests/keybinding_integration_tests.rs:1` で主要フローを検証済み。現状の課題はカスタムキーマップのホットリロード（将来フェーズ）。

## 見積もり
**期間**: 2-3日
**優先度**: 高（ユーザーインターフェース の核心）

## 関連タスク
- 01_architecture_design.md（前提）
- 04_minibuffer_design.md（M-x連携）
- keybinding_implementation.md（実装）

## 技術的考慮事項
- crossterm の KeyEvent 構造との整合性
- 将来のカスタマイズ機能への拡張性
- パフォーマンス（キー入力の応答性）
- メモリ効率（キーマップデータ構造）
