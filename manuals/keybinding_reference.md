# altre キーバインドリファレンス

## 表記について
- `C-` は `Ctrl` キー、`M-` は `Alt`（Meta）キーを表します。
- シーケンスは左から順番に入力します。例: `C-x C-s` は `Ctrl` + `x` → `Ctrl` + `s` の順。
- 記載のない場合は単一キーです。

## 1. 移動コマンド
| キー | コマンド | 説明 |
|------|----------|------|
| `C-f` | Forward Char | カーソルを 1 文字右へ移動 |
| `C-b` | Backward Char | カーソルを 1 文字左へ移動 |
| `C-n` | Next Line | カーソルを 1 行下へ移動 |
| `C-p` | Previous Line | カーソルを 1 行上へ移動 |
| `C-a` | Move Line Start | 現在行の先頭へ移動 |
| `C-e` | Move Line End | 現在行の末尾へ移動 |
| `M-<` | Beginning of Buffer | バッファの先頭へ移動 |
| `M->` | End of Buffer | バッファの末尾へ移動 |
| `↑` / `↓` / `←` / `→` | 矢印キー移動 | 方向キーで移動（端末互換） |

## 2. 編集コマンド
| キー | コマンド | 説明 |
|------|----------|------|
| 文字キー | Insert Char | 文字を挿入 |
| `Enter` | Insert Newline | 改行を挿入 |
| `Backspace` | Delete Backward Char | カーソル左の文字を削除 |
| `C-d` | Delete Char | カーソル右の文字を削除 |

## 3. ファイル操作
| キー | コマンド | 説明 |
|------|----------|------|
| `C-x C-f` | Find File | ファイルを開く。ミニバッファでパスを入力 |
| `C-x C-s` | Save Buffer | 現在のバッファを保存。未保存バッファは保存先入力へ遷移 |
| `C-x C-c` | Save Buffers Kill Terminal | altre を終了 |

## 4. ミニバッファとコマンド実行
| キー | コマンド | 説明 |
|------|----------|------|
| `M-x` | Execute Command | コマンド名を入力して実行 |
| `M-:` | Eval Expression | alisp 式を入力・評価 |
| `C-g` | Cancel | ミニバッファ操作をキャンセル（`MinibufferResult::Cancel`） |
| `Tab` | Complete | 補完候補を表示・選択 |

## 5. 保存関連コマンド
| コマンド | 推奨入力 | 解説 |
|----------|----------|------|
| `write-file` | `M-x write-file` | 別名保存。保存先を直接指定 |
| `save-buffer-as` | `M-x save-buffer-as` | `write-file` のエイリアス |
| `save-buffer` | `C-x C-s` | 変更がない場合は「変更なし」メッセージのみ表示 |

## 6. よくある質問
- **`M-x` を押しても何も起きない**: Alt キーが端末で `Meta` として送出されているか確認してください。必要に応じて `Esc` `x` の連打で代用できます。
- **`C-g` が効かない**: raw mode に対応していない端末で発生することがあります。別ターミナルを試してください。

## 7. 今後の拡張予定
- キーバインドのユーザー定義ファイル読込
- `describe-bindings` コマンドによる一覧表示
- マルチプレフィックスへの対応（例: `C-x 3` など）

このドキュメントは MVP 版の実装状況に基づきます。実装状況は `docs/design/minibuffer.md` および `app/src/input/keybinding.rs` を併せて参照してください。
