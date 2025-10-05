# TUIレイアウト設計

## タスク概要
ratatuiを使用したTUIの画面レイアウトとコンポーネント設計を行う。

## 目的
- MVPに必要な画面要素の配置設計
- ratatuiの効率的な活用方法の策定
- 将来のウィンドウ分割機能への拡張性確保
- レスポンシブな描画パフォーマンスの実現

## 設計対象
1. **画面レイアウト構成**
   - メインテキスト編集エリア
   - ミニバッファエリア
   - モードライン（簡易版）
   - 画面分割の基本単位

2. **描画コンポーネント**
   - テキストバッファ描画
   - カーソル表示
   - ミニバッファ描画
   - エラーメッセージ表示

3. **色彩設計**
   - 基本的な色設定
   - エラー表示色
   - カーソル強調色
   - 選択範囲表示（将来機能だが考慮）

4. **レスポンシブ対応**
   - ターミナルサイズ変更への対応
   - 最小表示サイズの定義
   - 文字折り返し処理

## 成果物
- `docs/design/tui_layout.md` - レイアウト設計書
- `app/src/ui/layout.rs` - レイアウト管理モジュール
- 画面モックアップ（ASCII アート）

## QA確認事項
- ターミナル最小サイズ要件
- 色対応レベル（256色、true color等）
- 日本語文字幅対応の必要性

## 完了条件
- [x] 画面レイアウト仕様の確定（`docs/design/tui_layout.md:1` にレイアウト仕様を記録）
- [x] ratatui コンポーネント設計の完成（`app/src/ui/layout.rs:1` と `app/src/ui/renderer.rs:1` を構成）
- [x] 色彩設計の策定（`app/src/ui/theme.rs:1` でテーマ設定を定義）
- [x] 描画パフォーマンス要件の設定（`docs/design/performance_tests_spec.md:11` に応答時間目標を明記）

## ステータス
- `app/tests/window_management_tests.rs:1` にて複数ウィンドウ描画の回帰テストを運用中。さらなる UI 改良は `tasks/done/20_search_replace_ui_ux_design.md` を参照。

## 見積もり
**期間**: 2-3日
**優先度**: 高（ユーザー体験の基盤）

## 関連タスク
- 01_architecture_design.md（前提）
- 04_minibuffer_design.md（ミニバッファ統合）
- tui_implementation.md（実装）

## 技術的考慮事項
- ratatui の Block, Paragraph, List ウィジェット活用
- crossterm での色対応確認
- UTF-8 文字の適切な幅計算
- 描画の最適化（差分更新等）
