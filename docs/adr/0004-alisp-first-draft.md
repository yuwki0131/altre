# ADR 0004: Altre Lisp 初期ドラフト

- **日付**: 2025-09-30
- **ステータス**: ドラフト
- **参照**: QA.md (Q1〜Q7)、docs/design/alisp_language_spec.md、docs/design/alisp_runtime_architecture.md

## 背景
MVP のエディタ機能が安定してきた段階で、長期的な拡張性を確保するためのスクリプト言語設計が必要になった。Emacs 由来の操作体系を踏まえ、同等の拡張性を持つ組み込み Lisp を早期に方向付けるため、本 ADR では初期ドラフトの方針と基本仕様を整理する。

## 概要
**Altre Lisp (alisp)** は、Emacs ライクなエディタ **Altre** に組み込まれる Lisp 方言である。エディタ拡張・設定・自動化を目的とし、Altre 起動後に常駐するインタプリタとして動作する。

- 設定言語・スクリプト言語・アプリ開発環境を兼ねる。
- ユーザー設定・拡張はすべて alisp で記述する。
- REPL による即時評価と動的再定義をサポートする。

## 言語仕様
### データ型
- **基本型**: シンボル / 数値 / 文字列 / リスト / ベクタ / ハッシュテーブル / ブール
- **名前空間**: Lisp-1（変数と関数は同一名前空間）

### 評価とスコープ
- **評価戦略**: S 式の逐次評価（先行評価）
- **スコープ**: レキシカルスコープ
- **定義**:
  - 変数・関数: `(define name expr)`
  - 無名関数: `(lambda (args) body)`
  - ローカル変数: `let`

### 実行
- `eval-buffer`、`eval-region`、`M-:` などで即時評価
- バイトコンパイル: `byte-compile-file` → `.elc`（内蔵バイトコードインタプリタで高速実行）
- 実行中の関数・変数を再定義可能

## Altre との統合
### ホスト環境
- Rust/C レイヤが低レベル機能（描画・ファイル I/O・GC など）を提供し、alisp はプリミティブ関数として利用する。

### プリミティブ関数の例
- `insert`、`buffer-name`、`split-window` など、低レベル API を直接呼び出す。

### インタラクティブ操作
- `interactive` 宣言によりキーボード操作から直接呼び出せるコマンドを定義する。

### フックとモード
- **フック**: 特定イベント発生時に実行される関数リスト（`add-hook`、`remove-hook`）。
- **モード**: バッファ単位の編集環境。メジャーモードは 1 バッファ 1 つ、マイナーモードは複数併用可能。

## 主要オブジェクト
| 概念 | 説明 | 主な関数例 |
|------|------|-----------|
| **バッファ** | テキスト保持単位。ファイルやメッセージもバッファ。 | `current-buffer`, `with-current-buffer` |
| **ウィンドウ** | 画面上にバッファを表示する枠 | `split-window`, `other-window` |
| **フレーム** | OS レベルのトップウィンドウ | `make-frame`, `delete-frame` |
| **ポイント/マーク** | カーソル位置と選択範囲 | `point`, `goto-char`, `set-mark` |
| **ミニバッファ** | コマンド・補完入力用 1 行バッファ | `read-string`, `read-from-minibuffer` |
| **プロセス** | 外部プログラムとの非同期通信 | `start-process`, `make-process` |
| **フェイス** | テキスト装飾（フォント・色） | `defface`, `face-attribute` |
| **キーマップ** | キーとコマンドの対応表 | `define-key`, `global-set-key` |
| **テキストプロパティ/オーバーレイ** | テキストにメタ情報を付加 | `put-text-property`, `overlay-put` |

## パッケージと初期化
- 初期化ファイル: `init.el`、`early-init.el` で設定・読み込みを行う。

## 典型コード例
### interactive な関数
```alisp
(define my-insert-date 'interactive
  (lambda ()
    "カーソル位置に現在の日付を挿入する"
    (insert (format-time-string "%Y-%m-%d"))))
```
- `define` で関数定義。
- `interactive` シンボルを付与するとキーバインド可能。
- `insert` は Rust 側プリミティブ関数。

### interactive でない通常の関数
```alisp
(define my-insert-date
  (lambda ()
    "カーソル位置に現在の日付を挿入する"
    (insert (format-time-string "%Y-%m-%d"))))
```

## まとめ
Altre Lisp は、Altre を単なるエディタ以上の「拡張可能な開発環境」へと進化させる中核言語である。設定・スクリプト・アプリ開発を一体化し、動的で強力な拡張性を提供する。
