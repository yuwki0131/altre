# 再帰呼出しがうまく機能していない

## 現状
- `docs/design/alisp_language_spec.md:65` は `(define (fn-name params...) body...)` で関数を定義できると記載しているが、現実装では `(define (fib n) ...)` 評価時にシンボル `fib` が未解決となりエラーが発生する。
- `src/alisp/evaluator.rs:123` の `define` 実装ではクロージャを作成してからシンボルを定義するため、クロージャ作成時点で自己参照が確立されていない。
- `tests/alisp_interpreter.rs:1` には再帰関数を検証するテストが存在せず、問題が自動検出されていない。

## 再現コード
```
(define (fib n)
  (if (<= n 1)
      n
      (+ (fib (- n 1)) (fib (- n 2)))))

(fib 10) ; => 55 を期待するが未定義エラー
```

## 次アクション
1. `eval_define` で関数定義を処理する際に、先に `define_symbol` で関数名にダミー値を設定し、クロージャ作成後に上書きする（自己参照を確保）。
2. `tests/alisp_interpreter.rs` に再帰評価シナリオを追加し、`fib`・`factorial` など複数ケースで回帰テストを作成。
3. GC への影響を確認し、`docs/adr-qa/alisp_QA.md` に再帰対応のメモとルート集合の更新内容を追記。
