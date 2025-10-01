# TODO: 単語単位の移動・削除コマンド実装

## 概要
Emacs 標準の単語単位移動・削除（`M-f` / `M-b` / `M-d` / `M-Backspace`）を altre に追加し、テキスト編集効率を向上させる。

## 背景
現在の altre には文字単位・行単位の移動のみが実装されており、単語単位の移動・削除が存在しない。Emacs の一般的な操作体系に追随するため早期導入が必要。

## 要件
- `M-f` / `M-b` で UTF-8 セーフな単語単位移動を実装する。
- `M-d` でカーソル以降の単語をキルリングへ格納する。
- `M-Backspace` でカーソル以前の単語をキルリングへ格納する。
- 単語境界の定義（ASCII のみか、Unicode の単語境界か）を決めて実装する。
- ミニバッファ・ユーザーガイドのドキュメントを更新する。

## 成果物
- `app/src/input/keybinding.rs` のバインド追加
- `app/src/input/commands.rs`/`app/src/editor`/`app/src/buffer` のロジック実装
- 主要テスト（ユニット／統合テスト）の追加
- `manuals/` や `docs/design/` の関連ドキュメント更新

## 参考
- INTRODUCE_EMACS_ADR.md（todo タグ / 2025-09-27 時点）
