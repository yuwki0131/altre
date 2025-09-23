# altre lisp first draft

# Altre Lisp 言語仕様・統合概要

## 1. 概要
**Altre Lisp (alisp)** は、Emacsライクなエディタ **Altre** に組み込まれるLisp方言です。
エディタ拡張・設定・自動化を目的とし、Altre起動後に常駐するインタプリタとして動作します。

* **役割**
  - 設定言語・スクリプト言語・アプリ開発環境を兼ねる。
  - ユーザー設定・拡張はすべてAltre Lispで記述可能。
  - REPLによる即時評価、動的再定義をサポート。

---

## 2. 言語仕様
### データ型
- **基本型**: シンボル / 数値 / 文字列 / リスト / ベクタ / ハッシュテーブル / ブール
- **名前空間**: Lisp-1（変数と関数は同一名前空間）

### 評価とスコープ
- **評価戦略**: S式を逐次評価（先行評価）
- **スコープ**: レキシカルスコープ
- **定義**:
  - 変数/関数: `(define name expr)`
  - 無名関数: `(lambda (args) body)`
  - ローカル変数: `let`

### 実行
- `eval-buffer`, `eval-region`, `M-:` などで即時評価
- バイトコンパイル: `byte-compile-file` → `.elc`
  内蔵バイトコードインタプリタで高速実行
- 実行中の関数・変数を再定義可能

---

## 3. Altreとの統合
### ホスト環境
- **構造**: Rust/Cレイヤが低レベル機能（描画・ファイルI/O・GCなど）を提供し、
  Altre Lisp はそれをプリミティブ関数として利用。

### プリミティブ関数例
- `insert`, `buffer-name`, `split-window` など、C実装の低レベルAPIを直接呼び出し可能。

### インタラクティブ操作
- `interactive` 宣言によりキーボード操作から直接呼び出せるコマンドを定義。

### フックとモード
- **フック**: 特定イベント発生時に実行される関数リスト。`add-hook`, `remove-hook`
- **モード**: バッファ単位の編集環境
  - メジャーモード: 1バッファ1つ
  - マイナーモード: 複数併用可

---

## 4. 主要オブジェクト
| 概念 | 説明 | 主な関数例 |
|------|------|-----------|
| **バッファ** | テキスト保持単位。ファイルやメッセージもバッファ。 | `current-buffer`, `with-current-buffer` |
| **ウィンドウ** | 画面上にバッファを表示する枠 | `split-window`, `other-window` |
| **フレーム** | OSレベルのトップウィンドウ | `make-frame`, `delete-frame` |
| **ポイント/マーク** | カーソル位置と選択範囲 | `point`, `goto-char`, `set-mark` |
| **ミニバッファ** | コマンド・補完入力用1行バッファ | `read-string`, `read-from-minibuffer` |
| **プロセス** | 外部プログラムとの非同期通信 | `start-process`, `make-process` |
| **フェイス** | テキスト装飾(フォント・色) | `defface`, `face-attribute` |
| **キーマップ** | キーとコマンドの対応表 | `define-key`, `global-set-key` |
| **テキストプロパティ/オーバーレイ** | テキストにメタ情報を付加 | `put-text-property`, `overlay-put` |

---

## 5. パッケージと初期化
- **初期化ファイル**: `init.el`, `early-init.el` で設定・読み込み

---

## 6. 典型コード例

### interactiveな関数

```alisp
(define my-insert-date 'interactive
  (lambda ()
    "カーソル位置に現在の日付を挿入する"
    (insert (format-time-string "%Y-%m-%d"))))
```

* define で関数定義
* defineスペシャルフォームに対してinteractiveシンボルでキーバインド可能
* insert はCレイヤのプリミティブ関数

### (interactiveでない)通常の関数


```alisp
(define my-insert-date
  (lambda ()
    "カーソル位置に現在の日付を挿入する"
    (insert (format-time-string "%Y-%m-%d"))))
```

* defineスペシャルフォームに対してinteractiveシンボルなしで一般的な関数を定義

## まとめ

Altre Lisp は、Altre を単なるエディタ以上の「拡張可能な開発環境」へと進化させる中核言語である。
設定・スクリプト・アプリ開発を一体化し、動的で強力な拡張性を提供する。
