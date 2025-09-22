# ファイル操作システム実装

## タスク概要
`docs/design/file_operations.md` に基づき、MVPに必要なファイル読み書きワークフロー（find-file / save-buffer）を実装し、ミニバッファおよびエディタ本体と統合する。

## 目的
- `C-x C-f` / `C-x C-s` を中心としたEmacs風ファイル操作を提供
- UTF-8およびLF改行へ統一された安全な入出力レイヤーを構築
- 読み込み・保存時のエラーをユーザーフレンドリーに通知
- 変更検出と保存確認の基盤を整備

## 実装対象
1. **パス処理レイヤー**
   - `PathProcessor` によるホーム展開・相対/絶対パス正規化
   - シンボリックリンク基本対応と無効パス検出
   - 現在ディレクトリを基点とした補完用 API
2. **メタデータ解析と検証**
   - `FileInfo` / `FileChangeTracker` の実装と統合
   - 権限・存在チェック、書き込み可否判定
   - 新規ファイル作成時の親ディレクトリ検証
3. **入出力パイプライン**
   - `FileReader` による読み込み・改行統一・BOM除去
   - `FileSaver` のアトミック保存処理と直列保存フォールバック
   - 変更検出と保存後の状態更新
4. **UI統合**
   - ミニバッファ find-file / save-buffer アクションからの呼び出し
   - 失敗時のエラーメッセージ表示（5秒間）
   - 成功時のフィードバックメッセージとバッファ更新
5. **テストと検証**
   - 正常系/異常系ユニットテスト（権限、存在、シンボリックリンク）
   - 変更検出ロジックのテスト
   - ミニバッファ経由の統合テスト（テンポラリディレクトリ利用）

## 成果物
- `app/src/file/{metadata,operations,path,io}.rs` の本実装
- `tests/file_operations/` または相当する統合テスト
- ミニバッファコマンドとの接続コード・メッセージ処理
- QA要件に沿ったエラーハンドリングの確認レポート

## 前提条件
- 07_file_operations_design.md の完了
- 02_error_handling_implementation.md（完了済）の利用
- ミニバッファシステム実装タスク（10）との調整

## 完了条件
- [ ] パス展開・正規化APIの実装およびテスト
- [ ] FileInfo / FileChangeTracker による検証機構の実装
- [ ] 読み込み・保存処理の実装とアトミック保存対応
- [ ] ミニバッファからの find-file / save-buffer 実行パスの確立
- [ ] 変更未検出時のスキップ・成功メッセージ表示の実装
- [ ] 主要異常系（権限不足・無効パス・I/O失敗）のテスト
- [ ] cargo test（該当モジュール）の成功

## 見積もり
**期間**: 2日
**優先度**: 高（基本機能）

## 関連タスク
- 03_gap_buffer_implementation.md（バッファ連携）
- 04_keybinding_implementation.md（`C-x C-f` / `C-x C-s` バインド）
- 10_minibuffer_system_implementation.md（UI統合）
- 12_performance_optimization_implementation.md（大ファイル時の性能確認）

## 技術的考慮事項
- UTF-8 / LF 正規化ポリシー（QA Q17/Q18）
- バックアップ無し方針（QA Q16）
- 変更検出のハッシュ計算コストとキャッシュ戦略
- エラー表示とログ出力の一貫性（ErrorDisplay / Logger の活用）
