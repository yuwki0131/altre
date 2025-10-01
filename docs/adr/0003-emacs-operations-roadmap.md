# 0003 - Emacs 標準操作導入ロードマップ

## 背景
altre は Emacs 互換の操作体系を目指しているが、現時点で実装済みのコマンドは限定的である。本ADRでは Emacs の代表的な操作を整理し、実装状況（done / todo / redesign / discard）と今後のタスクを明確化する。

---

## Emacs 標準操作一覧とタグ
タグ定義:
- `[done]` : altre で実装済み
- `[todo]` : 仕様をほぼ踏襲して導入予定
- `[redesign]` : altre 向けに要再設計
- `[discard]` : 当面導入しない

### 1. カーソル移動 (Motion)
- `C-f` / `C-b` : 1 文字進む / 戻る [done]
- `C-n` / `C-p` : 次 / 前の行へ移動 [done]
- `C-a` / `C-e` : 行頭 / 行末へ移動 [done]
- `M-f` / `M-b` : 単語単位で前後へ移動 [done]
- `M-<` / `M->` : バッファ先頭 / 末尾へ移動 [done]
- `C-l` : カーソルを画面中央/上/下に再配置 [done]
- `C-v` / `M-v` : 画面スクロール [done]
- `C-x <` / `C-x >` : 横スクロール [done]

### 2. 編集 (Editing)
- 通常文字入力 [done]
- `Backspace` / `DEL` : 1 文字削除 [done]
- `C-d` : 前方 1 文字削除 [done]
- `M-d` / `M-Backspace` : 単語削除 [done]
- `C-k` : 行末までキル [done]
- `C-y` : ヤンク [done]
- `M-y` : ヤンク履歴巡回 [done]
- `C-s` / `C-r` : インクリメンタル検索 [done]
- `M-%` : クエリ置換 [redesign]
- `C-/` (`C-_`) : Undo [redesign]
- `C-g` : 操作キャンセル [done]

### 3. マーク / リージョン操作 [todo]
- `C-space`, `C-w`, `M-w`, `C-x C-x`, `C-x h`

### 4. クリップボード (Kill Ring) [todo]
- `C-k` [done], `C-y` [done], `M-y` [done], `C-w` / `M-w` [todo]

### 5. ファイル操作 (File)
- `C-x C-f` : ファイルを開く [done]
- `C-x C-s` : ファイルを保存 [done]
- `C-x C-w` : 別名で保存 [todo]
- `C-x s` : すべてのバッファを保存 [todo]
- `C-x C-q` : toggle-read-only [discard]

### 6. バッファ / ウィンドウ管理
- バッファ: `C-x b`, `C-x k`, `C-x C-b` [todo]
- ウィンドウ: `C-x 2`, `C-x 3`, `C-x 1`, `C-x 0`, `C-x o` [todo]

### 7. ミニバッファ・コマンド
- `M-x` / `M-:` : 実装済 [done]
- `C-h` 系ヘルプ [discard]
- `TAB` 補完 [redesign]

### 8. モード切替・カスタマイズ [redesign]
- メジャーモード、マイナーモード、`customize`、`set-variable`

### 9. その他
- 終了 `C-x C-c` [done]
- 数値引数 `C-u`, `C-x C-0..9` [redesign]
- マクロ関連 [discard]
- ブックマーク [discard]

---

## Done タグ整合性確認（2025-09-27）
- 基本カーソル移動 (`C-f`/`C-b`/`C-n`/`C-p`) → `app/src/input/keybinding.rs:438-441`、`manuals/user_guide.md:52-61`
- インクリメンタル検索 (`C-s`/`C-r`) → `app/src/app.rs:190-214`、`app/src/search/mod.rs`、`manuals/user_guide.md:73-85`
- ファイル操作 (`C-x C-f`/`C-x C-s`) → `app/src/input/keybinding.rs:461-464`、`app/src/input/commands.rs:243-258`
- ミニバッファ (`M-x`/`M-:`) → `app/src/input/keybinding.rs:467-474`、`app/src/app.rs:274-320`
- 終了 `C-x C-c` → `app/src/input/keybinding.rs:463`、`app/src/input/commands.rs:287-306`
- 基本削除 (`Backspace`/`C-d`) → `app/src/input/keybinding.rs:456-458`
- `C-a`/`C-e` → `app/src/input/keybinding.rs:442-443`
- `M-<`/`M->` → `app/src/input/keybinding.rs:451-453`

## Todo タグ整合性確認（2025-09-27）
- 未実装: `M-f`/`M-b`, `M-d`/`M-Backspace`, スクロール系 (`C-l`, `C-v`, `M-v`, `C-x <`, `C-x >`), キルリング (`C-k`, `C-y`, `M-y`), `C-g`, リージョン操作, `C-x C-w`, `C-x s`, バッファ／ウィンドウ管理コマンド。
- 実装済みとして再分類: `C-a`/`C-e`, `M-<`/`M->`, `C-d`（ADR中で `done` に反映済み）。

---

## 今後のタスク（tasks/todo/functions/ 配下）
1. `01_word_motion_and_delete.md` — `M-f`/`M-b`/`M-d`/`M-Backspace`
2. `02_screen_scrolling_and_recenter.md` — `C-v`/`M-v`/`C-l`/`C-x <`/`C-x >`
3. `03_kill_ring_and_yank.md` — `C-k`/`C-y`/`M-y`
4. `04_region_selection_and_kill.md` — マークとリージョン操作
5. `05_global_cancel_command.md` — `C-g`
6. `06_extended_file_operations.md` — `C-x C-w`/`C-x s`
7. `07_buffer_management_commands.md` — `C-x b`/`C-x k`/`C-x C-b`
8. `08_window_management_commands.md` — `C-x 2`/`C-x 3`/`C-x 1`/`C-x 0`/`C-x o`

各タスクの詳細は対応する Markdown を参照。

---

## 決定事項
- `done` に分類された機能は現行コードとドキュメントで整合しており、追加作業不要。
- 上記 `todo` タスクを順次着手し、Emacs 操作との互換性を高める。
- `redesign` と `discard` 領域は別途 ADR で扱う。

---

## 参考
- `tasks/todo/functions/*.md`
- `manuals/user_guide.md`
- `app/src/input/keybinding.rs`, `app/src/app.rs`, `app/src/input/commands.rs`
