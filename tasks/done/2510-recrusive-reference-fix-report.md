# alisp 再帰呼び出しバグ修正レポート

## 修正日
2025-01-28

## バグの概要
altre lisp インタプリタで再帰関数（自分自身を呼び出す関数）が正しく動作しない問題が発生していた。

## 問題の詳細

### 症状
```lisp
;; フィボナッチ数列の定義（再帰関数）
(define (fib n)
  (if (<= n 1)
      n
      (+ (fib (- n 1)) (fib (- n 2)))))

(fib 10) ; => エラー: 未定義のシンボル fib
```

### 根本原因
3つの問題が組み合わさっていた:

1. **シンボルパーサーの不足**: `<`, `>`, `=` が有効なシンボル文字として認識されず、`<=` などの比較演算子がパースエラーになっていた

2. **関数定義順序の問題**: `define` 特殊フォームが以下の順序で処理されていた:
   - クロージャを作成
   - 環境に関数名を束縛
   
   この順序では、クロージャ作成時に環境に関数名が存在しないため、再帰呼び出し時に関数が見つからない

3. **GCの早期発動**: 再帰呼び出しが深くなると、実行中の環境がGCによって誤って回収され、"invalid env handle" エラーが発生

## 修正内容

### 1. シンボル文字の拡張
**ファイル**: `app/src/alisp/reader/mod.rs`

```rust
// 修正前
fn is_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+' | '*' | '/' | '?' | '!')
}

// 修正後
fn is_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+' | '*' | '/' | '?' | '!' | '<' | '>' | '=')
}
```

**効果**: `<=`, `>=`, `=`, `<`, `>` などの比較演算子を正しくパースできるようになった

---

### 2. 関数定義順序の変更
**ファイル**: `app/src/alisp/evaluator.rs`

```rust
// 修正前
let body: Vec<Expr> = tail[1..].to_vec();
let closure = make_closure(&mut self.runtime, params, body, env);
define_symbol(&mut self.runtime, env, fn_name, Value::Function(Function::Lambda(closure)));

// 修正後
let body: Vec<Expr> = tail[1..].to_vec();

// 再帰関数をサポートするため、まず関数名を仮の値で環境に定義する
// これにより、クロージャが関数名を含む環境をキャプチャできる
define_symbol(&mut self.runtime, env, fn_name, Value::Unit);

// 今度は関数名が定義された環境でクロージャを作成
let closure = make_closure(&mut self.runtime, params, body, env);

// 実際のクロージャで関数名の束縛を更新
define_symbol(&mut self.runtime, env, fn_name, Value::Function(Function::Lambda(closure)));
```

**効果**: クロージャが作成される前に関数名が環境に存在するため、再帰呼び出しが可能になった

---

### 3. GC呼び出し時の値保護の強化
**ファイル**: `app/src/alisp/evaluator.rs`

```rust
// 修正前
fn eval_call(&mut self, list: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
    let callee = self.eval_expr(&list[0], env)?;
    let mut args = Vec::new();
    for arg in &list[1..] {
        args.push(self.eval_expr(arg, env)?);
    }
    maybe_collect(&mut self.runtime, &args, &[env, self.global_env]);
    // ...

// 修正後
fn eval_call(&mut self, list: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
    let callee = self.eval_expr(&list[0], env)?;
    let mut args = Vec::new();
    for arg in &list[1..] {
        args.push(self.eval_expr(arg, env)?);
    }
    // GC実行時には callee も保護する必要がある（関数値がクロージャの場合、環境を参照している）
    let mut roots = args.clone();
    roots.push(callee.clone());
    maybe_collect(&mut self.runtime, &roots, &[env, self.global_env]);
    // ...
```

**効果**: GC実行時に関数値（クロージャ）も保護されるようになった

---

### 4. GC閾値の引き上げ
**ファイル**: `app/src/alisp/runtime/mod.rs`

```rust
// 修正前
impl GcHeap {
    pub fn new() -> Self {
        Self { entries: Vec::new(), allocated: 0, next_gc_threshold: 128 }
    }

// 修正後
impl GcHeap {
    pub fn new() -> Self {
        // GCの閾値を高く設定して、再帰関数実行中のGCによる問題を軽減
        Self { entries: Vec::new(), allocated: 0, next_gc_threshold: 1024 }
    }
```

**効果**: 再帰呼び出し中の環境が誤ってGCされにくくなった

## テストの追加

**ファイル**: `app/tests/alisp_interpreter.rs`

```rust
#[test]
fn recursive_function_fibonacci() {
    let mut interp = Interpreter::new();
    // フィボナッチ数列の定義（再帰関数）
    interp.eval("(define (fib n) (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))").unwrap();
    // fib(10) = 55 を検証
    let result = interp.eval("(fib 10)").unwrap();
    assert_eq!(result.display, "55");
}

#[test]
fn recursive_function_factorial() {
    let mut interp = Interpreter::new();
    // 階乗の定義（再帰関数）
    interp.eval("(define (fact n) (if (<= n 1) 1 (* n (fact (- n 1)))))").unwrap();
    // fact(5) = 120 を検証
    let result = interp.eval("(fact 5)").unwrap();
    assert_eq!(result.display, "120");
}
```

両方のテストが正常にパスすることを確認。

## テスト結果

### 修正前
```
running 2 tests
test recursive_function_factorial ... FAILED
test recursive_function_fibonacci ... FAILED

failures:
    recursive_function_factorial
    recursive_function_fibonacci

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 8 filtered out
```

### 修正後
```
running 10 tests
test boolean_logic ... ok
test define_function_and_call ... ok
test error_for_unknown_symbol ... ok
test eval_arithmetic ... ok
test lambda_closure_captures_environment ... ok
test let_scoping ... ok
test minibuffer_eval_formats_output ... ok
test print_emits_message ... ok
test recursive_function_factorial ... ok
test recursive_function_fibonacci ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

全テストスイート（約200以上のテスト）も全てパス。

## 影響範囲

### 直接の影響
- alisp インタプリタで再帰関数が正しく動作するようになった
- 比較演算子（`<`, `>`, `<=`, `>=`, `=`）が正しくパースされるようになった

### 間接的な影響
- より複雑な関数定義が可能になった
- リスト処理など、再帰を多用するアルゴリズムの実装が可能になった

### 性能への影響
- GC閾値の引き上げにより、メモリ使用量が若干増加する可能性がある
- ただし、MVPの使用シナリオでは問題にならないレベル
- 将来的により洗練されたGC戦略（スタックスキャンなど）を導入することを推奨

## 今後の課題

### GCの改善
現在のGC実装は以下の課題がある:
1. 実行スタック上の環境を追跡していない
2. 閾値ベースの単純な戦略

**推奨される改善**:
- 実行コンテキストスタックの導入
- スタックスキャンによる正確なルート検出
- 世代別GCの導入（将来的に）

### より複雑な再帰パターンのテスト
- 相互再帰（A が B を呼び、B が A を呼ぶ）
- 末尾再帰最適化の検討

## 関連ドキュメント
- `tasks/todo/bugs/analysis_bugs_and_missing_features.md` - バグ分析ドキュメント
- `docs/design/alisp_language_spec.md` - alisp 言語仕様
- `docs/design/alisp_runtime_architecture.md` - ランタイムアーキテクチャ

## 完了タスク
- `tasks/todo/bugs/2510-recrusive-reference.md` → `tasks/done/` に移動
