# alisp 処理系アーキテクチャ設計

## 背景
- QA.md:Q1〜Q7 で初期機能・評価経路・GC実装方針が確定。
- A_LISP_FIRST_DRAFT.md ではホスト統合の概念が示されているが、初期リリース向けの具体的な処理系設計が未整備。
- runtime 構成（値表現、環境、評価器、GC、プリミティブ境界）を定義しないと実装タスクを進められない。

## ゴール
- alisp v0 の実装アーキテクチャを `docs/design/alisp_runtime_architecture.md` として文書化。
- 選定する GC 方式、値レイアウト、環境モデル、最小プリミティブ群、エディタ統合ポイントを明文化。
- テスト戦略（単体・統合）と今後の拡張パスを概要レベルで整理。

## ToDo
- [x] 言語仕様(`docs/design/alisp_language_spec.md`) と整合するコンポーネント図・データフローを作成。
- [x] GC 方式とライフサイクル（割り込みタイミング、ルート集合）を決定し記述。
- [x] 環境チェーン・評価ステップ・エラー伝播の擬似コードをまとめる。
- [x] ミニバッファ評価経路と UI 連携の I/F を定義。

## 成果物
- `docs/design/alisp_runtime_architecture.md`

## 参考
- QA.md:Q1, Q2, Q5, Q6, Q7
- A_LISP_FIRST_DRAFT.md
- docs/architecture/mvp_architecture.md
