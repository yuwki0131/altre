# alisp v0 処理系アーキテクチャ設計

## 1. 概要
- 本書は alisp 初期実装 (v0) の処理系アーキテクチャを定義する。
- 仕様書 `docs/design/alisp_language_spec.md` を満たす評価系と、Rust ベースの Altre 本体との統合方法を記述する。
- 参照資料: QA.md (Q1〜Q7), A_LISP_FIRST_DRAFT.md, docs/architecture/mvp_architecture.md (全体構成)。

## 2. 設計方針
1. **最小構成**: ミニバッファからの即時評価のみを対象に、必須コンポーネントだけを構築する。
2. **分離可能性**: リーダー／評価器／ランタイム／プリミティブをモジュール分割し、将来の拡張 (リスト、マクロ、VM) に備える。
3. **安全性**: Rust の所有権モデルを活用し、メモリ安全性とエラー伝播を保証する。
4. **可観測性**: ログとテストフックを用意し、GC・評価の診断を行えるようにする。

## 3. モジュール構成
```
app/src/alisp/
├── mod.rs                 // エントリーポイント
├── reader/                // 字句解析・構文解析
├── ast.rs                 // AST 定義
├── runtime/
│   ├── value.rs           // 値表現
│   ├── env.rs             // 環境と束縛
│   ├── gc.rs              // GC 実装
│   └── printer.rs         // 値の文字列化
├── evaluator.rs           // AST 評価器
├── primitives.rs          // プリミティブ関数登録
└── integration/
    ├── minibuffer.rs      // ミニバッファ連携
    └── error.rs           // エラー型と変換
```
- テスト: `app/tests/alisp/` に統合テスト、各モジュールに単体テストを配置。

## 4. 処理パイプライン
1. **入力**: ミニバッファから UTF-8 文字列として alisp コードを取得。
2. **Reader**: 字句解析→構文解析で AST (`Expr`) を生成。位置情報を保持。
3. **Evaluator**: グローバル環境を参照しつつ AST を評価し、`Value` を返す。
4. **プリミティブ連携**: 必要な演算は `primitives::register_core` で登録した関数を呼び出す。
5. **出力**: 評価結果を `runtime::printer` で文字列化し、ミニバッファに表示。エラー時は `integration::error` を経由して通知。

## 5. Reader / AST
- Reader は 2 段構成: `Tokenizer` と `Parser`。
  - Tokenizer: 数値、文字列、記号、括弧、コメントを扱う。
  - Parser: 再帰下降でリスト構造を解析。v0 ではコード構造としてのみリストを生成し、ユーザー値には変換しない。
- AST (`Expr`) の主なバリアント:
  - `Expr::Number(NumberLiteral)` (`Integer` or `Float`)
  - `Expr::Boolean(bool)`
  - `Expr::String(String)`
  - `Expr::Symbol(SymbolId)`
  - `Expr::List(Vec<Expr>)`
- パーサは括弧の整合性、未対応リテラル、予約語チェックで `ReaderError` を返す。

## 6. 値表現 (`runtime::value`)
```rust
pub enum Value {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(GcHandle<String>),
    Function(Function)
}

pub enum Function {
    Builtin(fn(&mut EvalCtx, &[Value]) -> EvalResult<Value>),
    Lambda(LambdaClosure),
}

pub struct LambdaClosure {
    pub params: Vec<SymbolId>,
    pub body: Vec<Expr>,
    pub env: EnvHandle,
}
```
- 文字列・クロージャなどヒープ確保する値は GC ハンドルで管理。
- `()` (Unit) は `Value::Unit` を追加実装。

## 7. 環境モデル (`runtime::env`)
- 連結リスト型のイミュータブル環境。
```rust
pub struct Env {
    parent: Option<EnvHandle>,
    bindings: Vec<(SymbolId, ValueHandle)>,
}
```
- `EnvHandle` は GC 管理下の参照 (Arena インデックス or Rc-like) 。
- 操作
  - `extend(parent, bindings)` で新しいレキシカルフレームを生成。
  - ルックアップは現在のフレームから親方向へ線形探索。
  - `set!` は既存フレームを遡って更新。

## 8. 評価器 (`evaluator.rs`)
- `EvalCtx` で現在の環境、GC、エラー伝播を管理。
- 主な関数:
  - `eval(&mut EvalCtx, expr: &Expr) -> EvalResult<Value>`
  - `eval_list` / `apply_function`
- 特殊フォームは `match expr` で分岐し、専用ハンドラを実装。
- エラーは `EvalError` 列挙体。`integration::error` 経由で `AltreError` へ変換。
- `and` / `or` は短絡を実装。`begin` は順次評価に使用。

## 9. GC 設計 (`runtime::gc`)
- 方式: 単純な stop-the-world マーク&スイープ。
  - ルート集合: グローバル環境、現在の環境スタック、評価スタック (引数ベクタ)、プリミティブテーブル。
  - マーク: 深さ優先で到達可能な `GcHandle` をマーキング。
  - スイープ: 未マークオブジェクトを回収し、フリーリストに戻す。
- トリガ条件:
  - 新規割り当てでヒープ使用量がしきい値 (例: 128 KiB) を超えたとき。
  - 手動トリガ用 `gc_collect()` を `EvalCtx` 内に公開（テスト用）。
- 安全策:
  - ミニバッファ評価中は GC 中断を許可しない。
  - 将来のインタラクティブ連携を想定して GC 再入不可とする。

## 10. プリミティブ (`primitives.rs`)
- `register_core(env: &mut EnvHandle)` で標準関数を束縛。
- 実装は Rust 側の `fn(&mut EvalCtx, &[Value]) -> EvalResult<Value>`。
- 代表例:
  - `numeric::add`, `numeric::subtract`
  - `logic::and`, `logic::or`
  - `string::append`, `string::len`
  - `meta::type_of`, `meta::print`
- 返却値は GC ハンドル経由。文字列生成時は GC に登録。

## 11. ミニバッファ統合 (`integration/minibuffer.rs`)
- 仕事の流れ:
  1. `alisp-eval-expression` コマンドで入力文字列を取得。
  2. `alisp::evaluate(input: &str, app_ctx: &mut AppContext) -> EvalFeedback` を呼び出し。
  3. Reader→Evaluator→プリミティブのパイプラインを実行。
  4. `EvalFeedback::{Success(String), Error(String)}` を返し、UI 層で表示。
- ミニバッファ仕様との整合:
  - 成功時: `=> <結果>` 形式で 5 秒表示。
  - 失敗時: `alisp error: <種別> - <詳細>`。
  - 長文は複数行表示を許可 (仕様確認済み)。

## 12. エラー連携 (`integration/error.rs`)
- `EvalError` → `AltreError::Alisp(EvalError, SourceSpan)` に変換。
- ミニバッファ表示とログ出力を統一。
- Reader の位置情報を `SourceSpan { line, column }` として保持し、エラー表示に反映。

## 13. テスト戦略
- Reader: 正常系／異常系の単体テスト。括弧不一致、未知トークン等。
- Evaluator: 代表的な式の評価、エラー伝播をテーブルテスト化。
- GC: ストレステスト (大量オブジェクト生成→GC→再利用) を実施。
- 統合: `alisp-eval-expression` を通じた end-to-end テストを `app/tests/alisp/` に配置。
- ベンチマーク (任意): 数値演算、GC トリガ頻度の計測を `app/benches/` に追加検討。

## 14. 拡張ポイント
- リスト導入時: `Value::List(ConsHandle)` を追加し、Reader からの構築を許可。GC ルートも更新。
- マクロ導入時: AST へ `Expr::Quote`, `Expr::QuasiQuote` を追加し、評価器にマクロ展開フェーズを追加。
- VM 化: `evaluator` を bytecode compiler + VM に置換。現行 AST 評価器はフェイルセーフとして残す。
- プリミティブ追加: ホスト API (バッファ操作等) を `primitives` サブモジュールに整理して段階的に公開。

## 15. 開発・運用メモ
- ログ: `trace` レベルで Reader/Evaluator/GC のトレースを出力できるよう feature flag を用意。
- 診断: `EvalCtx` にヒープ統計 (オブジェクト数／最終 GC 時刻) を保持し `print` 経由で確認可能にする。
- エラーメッセージ: 日本語で記述し、ユーザー向けメッセージと開発者向け詳細を分離。

