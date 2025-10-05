# alisp リーダー/AST 実装

## 背景
- `docs/design/alisp_language_spec.md`（作成予定）に基づき、初期リリースで扱う S 式 subset をパースする必要がある。
- QA.md:Q4, Q6 で初期対応するリテラル・特殊フォームが限定されており、軽量なリーダー実装が求められる。

## ゴール
- `src/alisp/reader`（予定）にトークナイザとパーサを実装し、AST 構造体を定義する。
- 数値（整数/浮動小数）、シンボル、ブール、文字列、リスト構造（評価用の cons もしくは AST ノード）に対応。
- 無効入力時に適切なエラーを返し、単体テストを整備。

## ToDo
- [x] 言語仕様に沿ったトークン定義とエラー型を設計。
- [x] S 式 → AST 変換ロジックを実装し、位置情報を保持。
- [x] 単体テスト（代表的な式およびエラーケース）を追加。
- [x] ドキュメント/コメントを QA 要件に合わせて日本語で記述。

## 成果物
- `src/alisp/reader/mod.rs`
- `src/alisp/ast.rs`
- `tests/alisp_interpreter.rs`（リーダーの評価を含む結合テスト）

## 参考
- QA.md:Q4, Q6
- docs/adr/0004-alisp-first-draft.md
