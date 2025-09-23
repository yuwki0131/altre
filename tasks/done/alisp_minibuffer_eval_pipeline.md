# alisp ミニバッファ評価パイプライン統合

## 背景
- QA.md:Q7 で初期評価経路はミニバッファの即時実行のみと定義。
- 現行 MVP のミニバッファ仕様を踏まえ、alisp コードを入力→評価→結果表示するフローを統合する必要がある。

## ゴール
- ミニバッファから alisp コードを受け取り、リーダー→評価器→結果（またはエラー）表示までを接続。
- 成功時/失敗時のメッセージ表示を統一し、UI 仕様（docs/design/minibuffer.md）に沿った UX を実現。
- 結合テスト／手動確認手順を整備。

## ToDo
- [x] ミニバッファコマンド（例: `alisp-eval-expression`）を追加。
- [x] 評価結果をミニバッファへフォーマットして表示。
- [x] エラー時の表示・ログ出力を既存エラー処理に統合。
- [x] 結合テストまたは E2E テストを追加し、回帰テスト手順を文書化。

## 成果物
- `app/src/alisp/integration/minibuffer.rs`
- `app/src/alisp/integration/error.rs`
- `app/tests/alisp_interpreter.rs`

## 参考
- QA.md:Q1, Q6, Q7
- docs/design/minibuffer.md
