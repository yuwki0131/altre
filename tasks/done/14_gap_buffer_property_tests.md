# ギャップバッファ プロパティテスト実装

## タスク概要
`docs/design/gap_buffer_property_tests.md` の仕様に従い、ギャップバッファの不変条件を検証する proptest ベースのテストスイートを実装する。

## 目的
- ギャップバッファ構造の整合性を自動検証
- UTF-8 文字境界とデータ保持の安全性を保証
- 編集操作の回帰検知を自動化
- 未来の最適化や改変に対する信頼性向上

## 実装対象
1. **不変条件チェック**
   - ギャップ境界 (`gap_start <= gap_end <= len`) の検証
   - ギャップ外バッファが常に有効な UTF-8 であることの検証
   - 文字数・バイト数が期待どおりに変化することの検証
2. **操作プロパティ**
   - 挿入順序の独立性（操作順序を変えても結果が同一）
   - 削除操作の妥当性と期待文字の除去確認
   - 挿入と削除の逆操作プロパティ
3. **UTF-8 安全性プロパティ**
   - マルチバイト文字挿入時の正しい位置と妥当性確認
   - 文字境界を尊重した部分文字列取得の検証
4. **エラーハンドリングプロパティ**
   - 範囲外操作がエラーになること
   - 無効な境界での挿入/削除が拒否されること
5. **テスト基盤整備**
   - proptest 用ストラテジー（UTF-8 文字列、操作シーケンス）
   - 既存ユニットテストとの共存構成
   - テストケースの最小化と失敗時の診断ログ

## 成果物
- `tests/gap_buffer_prop.rs`（または `tests/` 配下）のプロパティテスト実装
- `proptest` ストラテジーと共通ヘルパーモジュール
- ドキュメント化された不変条件チェックリスト
- テスト実行ガイド（README 追記または docs/
  補足）

## 前提条件
- 03_gap_buffer_implementation.md の完了
- UTF-8 安全仕様の理解（docs/design/utf8_safety_spec.md）
- proptest クレートの利用準備

## 完了条件
- [x] ギャップ境界・UTF-8 整合性不変条件のテスト実装
- [x] 挿入/削除に関する操作プロパティの実装
- [x] エラーパス（境界外操作）のプロパティテスト実装
- [x] 生成データストラテジーの最適化（縮約が有効に働くこと）
- [x] `cargo test gap_buffer --offline` で安定動作
- [x] 失敗時にデバッグ可能なログ/アサートを用意

## 見積もり
**期間**: 1.5日
**優先度**: 中（品質保証）

## 関連タスク
- 09_basic_editing_core_implementation.md（編集操作との連携）
- 12_performance_optimization_implementation.md（ギャップ最適化の安全確認）
- 16_navigation_performance_tests.md（ナビゲーションとの整合性検証）

## 技術的考慮事項
- ランダム入力によるフレーキ動作を避けるためのシード管理
- 大入力ケースでのテスト時間と収束速度
- proptest のケース最小化に時間がかかる場合のタイムアウト設定
- 失敗時の再現のために `PROPTEST_CASE_SEED` をログ出力

## 備考
- `src/buffer/gap_buffer.rs` 内の `prop_random_operations_preserve_invariants` などで不変条件を網羅。
- 生成戦略は `operation_strategy` / `small_unicode_string` に集約し、テスト数は `cases: 256` に固定。
- 失敗時は `prop_assert!` の詳細メッセージと `PROPTEST_CASE_SEED` 環境変数で再現可能。
- 公開APIベースの回帰テストは `tests/gap_buffer_prop.rs` に配置。
