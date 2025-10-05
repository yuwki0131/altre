# Undo/Redo 設計

## 1. 目的
- MVP エディタに線形 Undo/Redo を導入し、安全な編集体験を提供する。
- `C-/` で Undo、`C-.` で Redo を実行できるようにする。
- 既存の編集パイプライン（`TextEditor` / `CommandProcessor` / `App`）との整合性を保つ。

## 2. 要件整理
- 履歴は **線形** に管理する。Undo 後に新しい編集が発生した場合は Redo 履歴を破棄する。
- 履歴は **バッファ単位** で保持し、バッファ破棄時に履歴も破棄する。
- Undo/Redo 実行時は **テキスト内容とカーソル位置** を復元する。
- 連続した文字入力/Backspace は **単語＋スペース単位でまとめて** 記録する。それ以外の操作はコマンド単位で記録する。
- 大きな編集への特別な最適化（スナップショット等）は MVP 範囲外。

## 3. 既存構成のおさらい
- `src/buffer/editor.rs` : Gap Buffer ベースの低レベル編集ロジック。`ChangeNotifier` を備える。
- `src/editor/text_editor.rs` : 入力バッファや性能計測を提供。最終的に `BaseTextEditor` (`buffer::editor::TextEditor`) を呼び出す。
- `src/input/commands.rs` : `CommandProcessor` が編集コマンドを実行。
- `src/app.rs` : `App` が `CommandProcessor` と `TextEditor` を統合し、複数バッファを管理。

Undo/Redo はバッファ (`OpenBuffer`) ごとに管理する必要があるため、`App` が `HistoryManager` を保持し、各 `OpenBuffer` に `HistoryStack` を保存する。

## 4. データモデル
### 4.1 HistoryEntry
```
pub struct HistoryEntry {
    pub operations: Vec<AtomicEdit>,
    pub cursor_before: CursorSnapshot,
    pub cursor_after: CursorSnapshot,
}
```

- `operations`: 1 回のコマンドで発生した編集（複数挿入/削除をまとめる）。
- `cursor_before` / `cursor_after`: Undo/Redo 時にカーソルを復元するための情報。

### 4.2 AtomicEdit
Undo/Redo で巻き戻す最小単位。以下を想定:
```
enum AtomicEdit {
    Insert { position: usize, text: String },
    Delete { position: usize, text: String },
}
```
- `position` は変更発生時のバッファ内文字インデックス。
- Undo 時は `Insert` → 削除、`Delete` → 挿入で逆操作を行う。

### 4.3 CursorSnapshot
```
struct CursorSnapshot {
    point: CursorPosition,
    mark: Option<usize>,
}
```
- `CursorPosition` は既存構造体を再利用。
- Mark は現在未活用だが将来対応のため保存しておく。

### 4.4 HistoryStack
各バッファが保持する履歴スタック:
```
struct HistoryStack {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}
```
- Undo 実行時: `undo.pop()` → 逆操作適用 → `redo.push(entry)`。
- Redo 実行時: `redo.pop()` → 正操作適用 → `undo.push(entry)`。
- 新しい編集が発生したら `redo.clear()`。

## 5. 履歴の記録タイミング
### 5.1 App でのフック
- `App::execute_command` 内で各編集コマンドの実行前後を把握できる。
- 編集開始時に `HistoryManager::begin_command` を呼び出し、終了時に `end_command` でコミットする。
- `HistoryRecorder` はコマンド種別を参照し、AtomicEdit の集合を `HistoryEntry` にまとめる。

### 5.2 ChangeNotifier の活用
- `buffer::editor::ChangeNotifier` は Insert/Delete/CursorMove を通知する。
- `HistoryRecorder` を ChangeListener として登録し、実際の編集内容・位置を取得する。
- `TextEditor` に文字挿入前の `flush_input_buffer()` があるため、ChangeEvent は実際の差分のみを報告する。

### 5.3 文字入力のまとめ
- `HistoryRecorder` は ChangeEvent::Insert を受け取るたびに連続性を判断し、単語+スペースごとに `operations` を統合する。
- 実装戦略:
  - Insert のたびに `buffer::editor::ChangeEvent::Insert` を受信。
    - 直前エントリが同じカーソル進行方向で、前回からの差分が単語境界と一致する場合は同一エントリに追記。
  - スペース文字を含む単語の境界判定は簡易的に「英数字/アンダースコアの連続 vs 非英数字」＋スペース扱いで分割。
- Backspace (`safe_delete_backward`) についても同様に単語+スペース単位でまとめる。Delete (前方) は単体操作とする。

## 6. Undo/Redo 実行フロー
1. `App` のコマンド処理で `Command::Undo` / `Command::Redo` を追加。
2. キーマップに `C-/` → Undo、`C-.` → Redo を登録（レイアウト差異に備えて `C-7` / `C-_` も許容）。
3. `App` が保持する `HistoryManager` を通じて Undo/Redo を実行し、現在の `TextEditor` に変更を適用。
4. `HistoryStack::apply_undo` / `apply_redo` は以下を行う:
   - `TextEditor` に対して逆操作を適用（Delete/Insert の逆呼び出し）。
   - 操作中は ChangeNotifier による履歴記録を一時的に無効化し、再帰的な記録を防ぐ。
   - 適用後、カーソルとマークを `cursor_before` / `cursor_after` で復元。

## 7. コンポーネント構成案
```
src/editor/history/
 ├── mod.rs             // HistoryStack, HistoryRecorder 公開
 ├── entry.rs           // HistoryEntry, AtomicEdit, CursorSnapshot
 └── recorder.rs        // ChangeListener 実装、まとめロジック
```

- `HistoryRecorder` が ChangeListener を実装し、ChangeEvent を受信。
- `HistoryStack` が undo/redo のスタック制御と適用ロジックを持つ。
- `App` が `HistoryManager` を保持し、バッファ切替時に `HistoryStack` をロードして履歴を更新する。

## 8. API 追加/変更まとめ
- `Command` enum に `Undo`, `Redo` を追加。
- `Action` enum (`src/input/keybinding.rs`) にも対応アクションを追加し、`C-/`, `C-.`（レイアウトによっては `C-7` / `C-_` / `C-?` / `C-\\` / `C-4` も許容）を割り当て。
- `App` に `HistoryManager` フィールドを追加し、バッファ切替時に `HistoryStack` を同期。
- `App` の編集系コマンドは `history.begin_command` / `end_command` でラップし、`HistoryRecorder` が `ChangeEvent` を受け取れるよう `TextEditor::add_change_listener` を利用。

## 9. 想定テスト
- 単純な挿入/削除の Undo/Redo。
- 単語入力（例: "this is"）をまとめて Undo できること。
- Undo → 新しい入力 → Redo がクリアされること。
- バッファ切替後に元バッファへ戻った際、履歴が維持されていること。
- Undo 時にカーソル位置が元に戻ること。

## 10. 今後の拡張余地
- 履歴サイズ上限の導入、スナップショット戦略の検討。
- ユーザー設定による履歴粒度調整。
- tree 型履歴や undo grouping の柔軟化（MVP 範囲外）。
