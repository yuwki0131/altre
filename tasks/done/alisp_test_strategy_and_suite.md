# alisp テスト戦略・自動化整備

## 背景
- 新規に導入する alisp 処理系は parser/evaluator/GC/統合など多層の品質確保が必要。
- QA.md ではテスト方針が明示されていないため、最低限の単体テスト/統合テスト計画を整備する必要がある。

## ゴール
- alisp 向けテスト戦略を策定し、`tests/alisp/` などにテストスイートを追加。
- parser/evaluator/primitives/GC/ミニバッファ統合について代表的なシナリオを自動化。
- 回帰テスト実行手順を README または専用ドキュメントに追記。

## ToDo
- [x] コンポーネント別テストケース一覧を作成。
- [x] Rust 単体テスト・統合テストを実装。
- [x] 必要に応じてプロパティテスト（例: 式の往復性）を検討。
- [x] テスト実行手順を docs/design/alisp_runtime_architecture.md などへ反映。

## 成果物
- `tests/alisp_interpreter.rs`
- `docs/design/alisp_runtime_architecture.md`（テスト戦略セクション更新）
- `cargo test` 実行ログ（作業記録）

## 参考
- QA.md 全般
- docs/design/gap_buffer_property_tests.md（テスト方針参考）
