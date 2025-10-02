# alisp v0 言語仕様書作成

## 背景
- QA.md:Q1〜Q7 で初期実装範囲（原始的な言語機能／データ型の限定／評価経路の最小化）が確定。
- `docs/adr/0004-alisp-first-draft.md` の概要を初期リリース仕様へ再整理する必要がある。
- 既存の設計群に alisp の詳細仕様が存在しなかったため、設計・実装前に正式な仕様書が必要。

## ゴール
- 初期版 alisp の言語仕様書（データ型、構文、特殊形式、評価規則、エラーモデル）を `docs/design/alisp_language_spec.md` として作成する。
- 仕様書に初期リリースで扱わない機能（リスト/ベクタ/ハッシュテーブル、マクロ、バイトコード等）を明記し、将来拡張のメモを添える。

## ToDo
- [x] 支援資料（QA.md、docs/adr/0004-alisp-first-draft.md）から初期機能を抽出して章立てを設計する。
- [x] データ型・リテラル表現・評価規則・特殊フォーム（`define`/`lambda`/`let` など）の詳細を記述。
- [x] エラークラスとメッセージ方針、未実装機能一覧を整理。
- [x] docs/design 配下に仕様書を配置し、レビュー用ノートを追記。

## 成果物
- `docs/design/alisp_language_spec.md`

## 参考
- QA.md:Q1, Q2, Q4, Q6, Q7
- docs/adr/0004-alisp-first-draft.md
