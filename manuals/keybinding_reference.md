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
| `M-f` | Forward Word | 次の単語末尾へ移動 |
| `M-b` | Backward Word | 前の単語先頭へ移動 |
| `C-v` | Scroll Page Down | 画面を下方向にスクロール |
| `M-v` | Scroll Page Up | 画面を上方向にスクロール |
| `C-l` | Recenter | カーソル行を中央→上→下の順に再配置 |
| `↑` / `↓` / `←` / `→` | 矢印キー移動 | 方向キーで移動（端末互換） |

## 2. 編集コマンド
| キー | コマンド | 説明 |
|------|----------|------|
| 文字キー | Insert Char | 文字を挿入 |
| `Enter` | Insert Newline | 改行を挿入 |
| `Backspace` | Delete Backward Char | カーソル左の文字を削除 |
| `C-d` | Delete Char | カーソル右の文字を削除 |
| `M-d` | Kill Word | カーソル以降の単語を削除しキルリングへ |
| `M-Backspace` | Backward Kill Word | カーソル以前の単語を削除しキルリングへ |
| `C-k` | Kill Line | カーソル位置から行末（改行を含む）まで削除しキルリングへ |
| `C-y` | Yank | キルリングの最新エントリを貼り付け |
| `M-y` | Yank Pop | 直前のヤンクを次のエントリで置き換え |
| `C-x <` | Scroll Left | 水平スクロール（右側のテキストを表示） |
| `C-x >` | Scroll Right | 水平スクロール（左側のテキストを表示） |

## 3. ファイル操作
| キー | コマンド | 説明 |
|------|----------|------|
| `C-x C-f` | Find File | ファイルを開く。ミニバッファでパスを入力 |
| `C-x C-s` | Save Buffer | 現在のバッファを保存。未保存バッファは保存先入力へ遷移 |
| `C-x C-w` | Write File | 別名でファイルを保存。保存先をミニバッファで指定 |
| `C-x s` | Save Some Buffers | すべてのバッファを保存（現状は単一バッファに対して保存処理を実行） |
| `C-x C-c` | Save Buffers Kill Terminal | altre を終了 |

## 4. ミニバッファとコマンド実行
| キー | コマンド | 説明 |
|------|----------|------|
| `M-x` | Execute Command | コマンド名を入力して実行 |
| `M-:` | Eval Expression | alisp 式を入力・評価 |
| `C-g` | Keyboard Quit | 進行中の操作をキャンセルし、メッセージを表示 |
| `Tab` | Complete | 補完候補を表示・選択 |

## 5. 保存関連コマンド
| コマンド | 推奨入力 | 解説 |
|----------|----------|------|
| `write-file` | `M-x write-file` | 別名保存。保存先を直接指定 |
| `save-buffer-as` | `M-x save-buffer-as` | `write-file` のエイリアス |
| `save-buffer` | `C-x C-s` | 変更がない場合は「変更なし」メッセージのみ表示 |

## 6. ウィンドウ操作
| キー | コマンド | 説明 |
|------|----------|------|
| `C-x 2` | Split Window Below | 現在のウィンドウを上下に分割し、同じバッファを共有表示 |
| `C-x 3` | Split Window Right | 現在のウィンドウを左右に分割 |
| `C-x 1` | Delete Other Windows | フォーカス中ウィンドウ以外をすべて閉じる |
| `C-x 0` | Delete Window | フォーカス中ウィンドウを閉じ、残りのウィンドウへ切り替え |
| `C-x o` | Other Window | 次のウィンドウにフォーカスを移動 |

> 注記: 現時点ではすべてのウィンドウが同一バッファを共有します。バッファ単位の表示切替は今後の改良項目です。

## 7. よくある質問
- **`M-x` を押しても何も起きない**: Alt キーが端末で `Meta` として送出されているか確認してください。必要に応じて `Esc` `x` の連打で代用できます。
- **`C-g` が効かない**: raw mode に対応していない端末で発生することがあります。別ターミナルを試してください。

## 8. 今後の拡張予定
- キーバインドのユーザー定義ファイル読込
- `describe-bindings` コマンドによる一覧表示
- バッファごとのウィンドウ表示切替

このドキュメントは MVP 版の実装状況に基づきます。実装状況は `docs/design/minibuffer.md` および `app/src/input/keybinding.rs` を併せて参照してください。
