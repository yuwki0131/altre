# alisp 評価器・環境実装

## 背景
- QA.md:Q1, Q6, Q7 により初期評価機構がミニバッファ経由の即時実行に限定され、必要な特殊フォームも限定的。
- 処理系アーキテクチャ設計で定めた環境モデルと一致する evaluator が必要。

## ゴール
- `src/alisp/evaluator` に AST 評価器と環境チェーンを実装。
- `define` / `lambda` / `let` / 関数呼び出し / 条件分岐（必要に応じて）など仕様で定めた最小特殊フォームをサポート。
- エラー伝播を `AltreError` と統合し、単体テストを整備。

## ToDo
- [x] 環境（グローバル/レキシカル）のデータ構造を実装。
- [x] AST ノードごとの評価ロジックとエラー処理を実装。
- [x] 単体テストと代表的な評価シナリオを作成。
- [x] ドキュメントコメントを追加し、将来拡張の TODO を記載。

## 成果物
- `src/alisp/evaluator.rs`
- `src/alisp/runtime/mod.rs`
- `tests/alisp_interpreter.rs`

## 参考
- QA.md:Q1, Q2, Q6, Q7
- docs/design/alisp_runtime_architecture.md（作成予定）
