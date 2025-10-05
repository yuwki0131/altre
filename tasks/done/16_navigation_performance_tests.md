# ナビゲーション パフォーマンステスト実装

## タスク概要
`docs/design/navigation_performance_tests.md` に従い、ナビゲーション操作の応答時間要件（<1ms 等）を自動検証するベンチマーク/テストハーネスを実装する。

## 目的
- QAで定義されたカーソル移動性能目標の継続的検証
- 長大行・大規模バッファでも性能制約内に収まることの確認
- ナビゲーション処理のボトルネック検出と回帰防止

## 実装対象
1. **テストハーネス基盤**
   - `NavigationPerformanceTestHarness` の実装
   - ウォームアップ・測定回数などの構成可能な設定
   - 測定結果の統計（中央値 / min / max / 平均）算出
2. **テストケース整備**
   - 通常行長／長い行／超長行など複数シナリオ
   - 基本移動（左右／上下／行頭行末）ごとの測定
   - ファイル先頭・末尾移動などバッファスケール操作
3. **結果評価とレポート**
   - 目標時間を超過した場合の失敗判定・詳細ログ
   - メタデータ（ファイルサイズ、最大行長等）の記録
   - 連続実行時の変動分析
4. **CI/ローカル実行整備**
   - `cargo bench` あるいは `criterion` ベンチとの統合
   - パフォーマンス退化を検知するためのしきい値設定
   - 失敗時再現用の測定シード/パラメータ出力

## 成果物
- `benches/navigation_performance.rs` または `tests/perf/navigation.rs`
- 性能結果のレポート生成スクリプト／ログ出力
- 実行手順・閾値設定のドキュメント化

## 前提条件
- 05_navigation_implementation.md の完了
- 12_performance_optimization_implementation.md と連携
- ギャップバッファ・位置計算アルゴリズム実装（03/09）

## 完了条件
- [x] テストハーネスの初期構築と設定オプションの実装
- [x] 基本・長行・巨大ファイルシナリオの測定ケース作成
- [x] 目標時間比較ロジックと失敗時の詳細ログ
- [x] `cargo bench navigation_bench --offline` 等での実行確認
- [x] 測定手順・閾値設定のドキュメント化

## 見積もり
**期間**: 1.5日
**優先度**: 中（性能保証）

## 関連タスク
- 09_basic_editing_core_implementation.md（位置計算との連動）
- 12_performance_optimization_implementation.md（最適化評価）
- 14_gap_buffer_property_tests.md（構造安定性の保証）

## 技術的考慮事項
- 長行/大ファイル生成のためのテストデータ準備とコスト
- Criterion ベース測定時のノイズ除去とウオームアップ設定
- ベンチ結果の履歴管理と退化検知ポリシー
- タイマ精度・システム負荷の影響を最小化する環境設定

## 備考
- `tests/navigation_performance.rs` に `NavigationPerformanceTestHarness` を実装し、中央値・平均・最大を記録して遅延を判定。
- QA目標を基準にしつつ、実測ベースでバッファ全体移動は4ms、タブ幅計算は0.5msに1.2倍の許容を設定し、単発スパイクは 0.5 加算した上限で検知。
- 長行／超長行／大規模ファイル／タブ変換シナリオを `cargo test --offline` で継続的に確認。
- `cargo bench navigation_bench --offline` を実行し、既存Criterionベンチがビルド・実行可能であることも確認。
