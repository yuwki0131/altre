# 基本編集設計仕様書

## 概要

本文書は、Altreテキストエディタの基本編集機能（文字入力、削除、改行）の詳細設計を定義する。ギャップバッファと統合し、UTF-8文字境界を考慮した安全で高性能な編集操作を実現する。

## 設計目標

1. **高性能**: カーソル位置での操作を1ms未満で実行（QA.mdの要件）
2. **UTF-8安全性**: 文字境界での安全な操作保証
3. **統合性**: ギャップバッファ、カーソル管理、キーバインドとの連携
4. **拡張性**: 将来のアンドゥ・リドゥ機能への対応

## アーキテクチャ概要

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   KeyBinding    │───▶│   EditActions   │───▶│   GapBuffer     │
│   (キー入力)     │    │   (編集操作)     │    │  (データ格納)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │ CursorPosition  │
                       │  (位置管理)      │
                       └─────────────────┘
```

## 1. テキスト入力システム設計

### 1.1 文字入力処理

```rust
/// 編集操作インターフェース
pub trait EditOperations {
    /// 文字を挿入
    fn insert_char(&mut self, ch: char) -> Result<(), EditError>;

    /// 文字列を挿入
    fn insert_str(&mut self, s: &str) -> Result<(), EditError>;

    /// カーソル位置に安全に文字を挿入
    fn insert_char_at_cursor(&mut self, ch: char) -> Result<(), EditError>;
}

/// 文字入力の実装
impl EditOperations for TextEditor {
    fn insert_char(&mut self, ch: char) -> Result<(), EditError> {
        // 1. 入力文字の検証
        if !self.is_valid_input_char(ch) {
            return Err(EditError::InvalidChar(ch));
        }

        // 2. カーソル位置の取得
        let cursor_pos = self.cursor.char_pos;

        // 3. ギャップバッファに挿入
        self.buffer.insert(cursor_pos, ch)?;

        // 4. カーソル位置を更新
        self.cursor.move_forward(&self.buffer.to_string());

        // 5. 変更通知
        self.notify_change(ChangeEvent::Insert {
            position: cursor_pos,
            content: ch.to_string(),
        });

        Ok(())
    }
}
```

### 1.2 入力文字検証

```rust
impl TextEditor {
    /// 有効な入力文字かどうかを判定
    fn is_valid_input_char(&self, ch: char) -> bool {
        match ch {
            // 制御文字は除外（改行は別途処理）
            '\u{0000}'..='\u{001F}' => false,
            '\u{007F}' => false, // DEL
            // 印刷可能文字とスペース、タブは有効
            _ => !ch.is_control() || ch == '\t'
        }
    }

    /// UTF-8として安全な文字かどうかを検証
    fn is_safe_utf8_char(&self, ch: char) -> bool {
        // UTF-8として正常にエンコードできるかチェック
        let mut buf = [0; 4];
        ch.encode_utf8(&mut buf);
        true // Rustのcharは常に有効なUnicode
    }
}
```

### 1.3 IME統合考慮（MVP範囲外）

```rust
/// 将来のIME統合インターフェース
pub trait IMEHandler {
    /// IME入力開始
    fn start_ime_input(&mut self) -> Result<(), EditError>;

    /// IME変換中テキストの更新
    fn update_ime_composition(&mut self, text: &str) -> Result<(), EditError>;

    /// IME入力確定
    fn commit_ime_input(&mut self, text: &str) -> Result<(), EditError>;

    /// IME入力キャンセル
    fn cancel_ime_input(&mut self) -> Result<(), EditError>;
}

// MVPでは基本的な文字入力のみ実装
// IMEは将来バージョンで対応
```

### 1.4 入力バッファリング戦略

```rust
/// 高速な連続入力に対応するバッファリング
pub struct InputBuffer {
    /// 入力待ちの文字列
    pending_chars: String,
    /// 最後の入力時刻
    last_input_time: Instant,
    /// バッファリング閾値（1ms）
    buffer_timeout: Duration,
}

impl InputBuffer {
    /// 文字を入力バッファに追加
    pub fn add_char(&mut self, ch: char) {
        self.pending_chars.push(ch);
        self.last_input_time = Instant::now();
    }

    /// バッファの内容をフラッシュすべきかチェック
    pub fn should_flush(&self) -> bool {
        !self.pending_chars.is_empty() &&
        self.last_input_time.elapsed() > self.buffer_timeout
    }

    /// バッファの内容を取得してクリア
    pub fn flush(&mut self) -> String {
        std::mem::take(&mut self.pending_chars)
    }
}
```

## 2. 削除操作設計

### 2.1 削除操作インターフェース

```rust
/// 削除操作の種類
#[derive(Debug, Clone, PartialEq)]
pub enum DeleteOperation {
    /// Backspace: カーソル前の文字を削除
    Backward,
    /// Delete: カーソル後の文字を削除
    Forward,
    /// 範囲削除
    Range { start: usize, end: usize },
}

impl EditOperations for TextEditor {
    /// Backspace削除（カーソル前削除）
    fn delete_backward(&mut self) -> Result<char, EditError> {
        if self.cursor.char_pos == 0 {
            return Err(EditError::AtBufferStart);
        }

        let pos = self.cursor.char_pos - 1;
        let deleted_char = self.buffer.delete(pos)?;

        // カーソルを後退
        self.cursor.move_backward(&self.buffer.to_string());

        // 変更通知
        self.notify_change(ChangeEvent::Delete {
            position: pos,
            content: deleted_char.to_string(),
        });

        Ok(deleted_char)
    }

    /// Delete削除（カーソル後削除）
    fn delete_forward(&mut self) -> Result<char, EditError> {
        if self.cursor.char_pos >= self.buffer.len_chars() {
            return Err(EditError::AtBufferEnd);
        }

        let pos = self.cursor.char_pos;
        let deleted_char = self.buffer.delete(pos)?;

        // カーソル位置は変更なし（文字が削除されたため相対的に進む）

        // 変更通知
        self.notify_change(ChangeEvent::Delete {
            position: pos,
            content: deleted_char.to_string(),
        });

        Ok(deleted_char)
    }
}
```

### 2.2 UTF-8文字境界での安全な削除

```rust
impl TextEditor {
    /// UTF-8文字境界での安全な削除を保証
    fn safe_delete_at_position(&mut self, pos: usize) -> Result<char, EditError> {
        // 1. 位置が文字境界かチェック
        if !self.is_char_boundary(pos) {
            return Err(EditError::NotCharBoundary(pos));
        }

        // 2. 範囲チェック
        if pos >= self.buffer.len_chars() {
            return Err(EditError::OutOfBounds(pos));
        }

        // 3. 削除対象文字の取得
        let deleted_char = self.get_char_at(pos)?;

        // 4. ギャップバッファから削除
        self.buffer.delete(pos)?;

        Ok(deleted_char)
    }

    /// 指定位置が文字境界かどうかを判定
    fn is_char_boundary(&self, pos: usize) -> bool {
        // ギャップバッファの文字位置は常に文字境界
        // （内部でUTF-8の文字単位で管理）
        pos <= self.buffer.len_chars()
    }

    /// 指定位置の文字を取得
    fn get_char_at(&self, pos: usize) -> Result<char, EditError> {
        let text = self.buffer.to_string();
        text.chars().nth(pos).ok_or(EditError::OutOfBounds(pos))
    }
}
```

### 2.3 複合文字対応考慮（将来実装）

```rust
/// 将来の複合文字対応（結合文字等）
pub trait CompositeCharHandler {
    /// 複合文字の境界を判定
    fn is_grapheme_boundary(&self, pos: usize) -> bool;

    /// グラフェムクラスタ単位での削除
    fn delete_grapheme_backward(&mut self) -> Result<String, EditError>;
    fn delete_grapheme_forward(&mut self) -> Result<String, EditError>;
}

// MVPでは基本的なUTF-8文字単位の削除のみ実装
// 複合文字（結合文字、異体字セレクタ等）は将来対応
```

## 3. 改行処理設計

### 3.1 改行挿入操作

```rust
impl EditOperations for TextEditor {
    /// 改行を挿入
    fn insert_newline(&mut self) -> Result<(), EditError> {
        let cursor_pos = self.cursor.char_pos;

        // LF統一ポリシー（QA.mdの方針）
        self.buffer.insert_str(cursor_pos, "\n")?;

        // カーソルを次の行の先頭に移動
        self.cursor.move_forward(&self.buffer.to_string());

        // 変更通知
        self.notify_change(ChangeEvent::Insert {
            position: cursor_pos,
            content: "\n".to_string(),
        });

        Ok(())
    }
}
```

### 3.2 LF統一ポリシー

```rust
/// 改行コードの正規化
impl TextEditor {
    /// 入力された改行を正規化
    fn normalize_line_ending(&self, input: &str) -> String {
        input
            .replace("\r\n", "\n")  // Windows CRLF → LF
            .replace("\r", "\n")    // Mac CR → LF
    }

    /// ファイル読み込み時の改行正規化
    fn normalize_file_content(&self, content: &str) -> String {
        self.normalize_line_ending(content)
    }

    /// ファイル保存時は改行をそのまま維持（LF）
    fn prepare_for_save(&self, content: &str) -> String {
        content.to_string() // LFのまま保存
    }
}
```

### 3.3 自動インデント（MVP対象外、設計のみ）

```rust
/// 将来の自動インデント機能
pub trait AutoIndentHandler {
    /// 前の行のインデントを取得
    fn get_previous_line_indent(&self, line: usize) -> String;

    /// 自動インデント付き改行
    fn insert_newline_with_indent(&mut self) -> Result<(), EditError>;

    /// インデントレベルを計算
    fn calculate_indent_level(&self, line: usize) -> usize;
}

// MVPでは基本的な改行挿入のみ実装
// 自動インデントは将来バージョンで対応
```

## 4. カーソル状態管理

### 4.1 カーソル位置の追跡・更新

```rust
/// エディタ状態管理
pub struct TextEditor {
    /// テキストバッファ
    buffer: GapBuffer,
    /// カーソル位置
    cursor: CursorPosition,
    /// 変更通知システム
    change_notifier: ChangeNotifier,
}

impl TextEditor {
    /// カーソル位置をギャップバッファと同期
    fn sync_cursor_with_buffer(&mut self) {
        let text = self.buffer.to_string();

        // カーソル位置の有効性をチェック
        if self.cursor.char_pos > self.buffer.len_chars() {
            // バッファ末尾に修正
            self.cursor.char_pos = self.buffer.len_chars();
        }

        // 行・列情報を再計算
        self.recalculate_cursor_line_column(&text);
    }

    /// カーソルの行・列位置を再計算
    fn recalculate_cursor_line_column(&mut self, text: &str) {
        let mut line = 0;
        let mut column = 0;

        for (i, ch) in text.chars().enumerate() {
            if i == self.cursor.char_pos {
                break;
            }

            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        self.cursor.line = line;
        self.cursor.column = column;
    }
}
```

### 4.2 ギャップバッファとの位置同期

```rust
impl TextEditor {
    /// ギャップバッファのギャップ位置とカーソルを同期
    fn sync_gap_with_cursor(&mut self) -> Result<(), EditError> {
        let cursor_byte_pos = self.char_to_byte_position(self.cursor.char_pos)?;
        self.buffer.move_gap_to(cursor_byte_pos)?;
        Ok(())
    }

    /// 文字位置をバイト位置に変換
    fn char_to_byte_position(&self, char_pos: usize) -> Result<usize, EditError> {
        let text = self.buffer.to_string();
        let char_indices: Vec<_> = text.char_indices().collect();

        if char_pos > char_indices.len() {
            return Err(EditError::OutOfBounds(char_pos));
        }

        if char_pos == char_indices.len() {
            Ok(text.len())
        } else {
            Ok(char_indices[char_pos].0)
        }
    }
}
```

### 4.3 境界値処理

```rust
impl TextEditor {
    /// カーソル移動の境界値チェック
    fn clamp_cursor_position(&mut self) {
        let max_pos = self.buffer.len_chars();

        if self.cursor.char_pos > max_pos {
            self.cursor.char_pos = max_pos;
        }

        // 行・列の境界値も調整
        let text = self.buffer.to_string();
        self.clamp_cursor_line_column(&text);
    }

    /// 行・列位置の境界値チェック
    fn clamp_cursor_line_column(&mut self, text: &str) {
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            self.cursor.line = 0;
            self.cursor.column = 0;
            return;
        }

        // 行数の境界値
        if self.cursor.line >= lines.len() {
            self.cursor.line = lines.len() - 1;
        }

        // 列数の境界値
        if self.cursor.line < lines.len() {
            let line_length = lines[self.cursor.line].chars().count();
            if self.cursor.column > line_length {
                self.cursor.column = line_length;
            }
        }
    }
}
```

## 5. 編集操作の統合

### 5.1 アンドゥ・リドゥ準備

```rust
/// 編集操作の記録（将来のアンドゥ・リドゥ用）
#[derive(Debug, Clone)]
pub enum EditCommand {
    Insert {
        position: usize,
        content: String,
    },
    Delete {
        position: usize,
        content: String,
    },
    Replace {
        position: usize,
        old_content: String,
        new_content: String,
    },
}

/// コマンド履歴管理（将来実装）
pub struct CommandHistory {
    /// 実行済みコマンド
    undo_stack: Vec<EditCommand>,
    /// 取り消し済みコマンド
    redo_stack: Vec<EditCommand>,
    /// 履歴の最大サイズ
    max_history_size: usize,
}

impl CommandHistory {
    /// コマンドを記録
    pub fn record_command(&mut self, command: EditCommand) {
        self.undo_stack.push(command);
        self.redo_stack.clear(); // 新しい操作でリドゥ履歴をクリア

        // 履歴サイズの制限
        if self.undo_stack.len() > self.max_history_size {
            self.undo_stack.remove(0);
        }
    }
}
```

### 5.2 変更通知システム

```rust
/// 変更イベント
#[derive(Debug, Clone)]
pub enum ChangeEvent {
    Insert {
        position: usize,
        content: String,
    },
    Delete {
        position: usize,
        content: String,
    },
    CursorMove {
        old_position: CursorPosition,
        new_position: CursorPosition,
    },
}

/// 変更通知システム
pub struct ChangeNotifier {
    /// イベントリスナー
    listeners: Vec<Box<dyn ChangeListener>>,
}

pub trait ChangeListener {
    fn on_change(&mut self, event: &ChangeEvent);
}

impl ChangeNotifier {
    /// リスナーを追加
    pub fn add_listener(&mut self, listener: Box<dyn ChangeListener>) {
        self.listeners.push(listener);
    }

    /// 変更を通知
    pub fn notify(&mut self, event: ChangeEvent) {
        for listener in &mut self.listeners {
            listener.on_change(&event);
        }
    }
}
```

### 5.3 リアルタイム表示更新

```rust
/// 表示更新マネージャー
pub struct DisplayUpdateManager {
    /// 最後の更新時刻
    last_update: Instant,
    /// 更新間隔（16ms = 60FPS）
    update_interval: Duration,
    /// 更新が必要かのフラグ
    needs_update: bool,
}

impl ChangeListener for DisplayUpdateManager {
    fn on_change(&mut self, event: &ChangeEvent) {
        self.needs_update = true;

        // 緊急更新が必要な場合（カーソル移動など）
        match event {
            ChangeEvent::CursorMove { .. } => {
                self.schedule_immediate_update();
            }
            _ => {
                self.schedule_deferred_update();
            }
        }
    }
}

impl DisplayUpdateManager {
    /// 即座に更新をスケジュール
    fn schedule_immediate_update(&mut self) {
        if self.last_update.elapsed() > Duration::from_millis(1) {
            self.force_update();
        }
    }

    /// 遅延更新をスケジュール
    fn schedule_deferred_update(&mut self) {
        // バッチ更新のために少し待機
    }

    /// 強制的に更新
    fn force_update(&mut self) {
        if self.needs_update {
            // TUI更新処理を呼び出し
            self.last_update = Instant::now();
            self.needs_update = false;
        }
    }
}
```

## 6. エラーハンドリング

### 6.1 編集エラー定義

```rust
/// 編集操作エラー
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("Position {0} is out of bounds")]
    OutOfBounds(usize),

    #[error("Position {0} is not on a character boundary")]
    NotCharBoundary(usize),

    #[error("Invalid character: {0:?}")]
    InvalidChar(char),

    #[error("Cursor at buffer start")]
    AtBufferStart,

    #[error("Cursor at buffer end")]
    AtBufferEnd,

    #[error("Buffer operation failed: {0}")]
    BufferError(#[from] BufferError),

    #[error("Memory allocation failed")]
    OutOfMemory,

    #[error("Operation cancelled")]
    Cancelled,
}
```

### 6.2 エラー復旧戦略

```rust
impl TextEditor {
    /// エラー時の安全な状態復旧
    fn recover_from_error(&mut self, error: &EditError) -> Result<(), EditError> {
        match error {
            EditError::OutOfBounds(_) | EditError::NotCharBoundary(_) => {
                // カーソル位置を安全な位置に修正
                self.clamp_cursor_position();
                Ok(())
            }
            EditError::BufferError(_) => {
                // バッファとカーソルの整合性を回復
                self.sync_cursor_with_buffer();
                Ok(())
            }
            EditError::OutOfMemory => {
                // 致命的エラー：QA.mdに従い即座に終了
                Err(EditError::OutOfMemory)
            }
            _ => Ok(())
        }
    }

    /// 操作の安全実行
    fn safe_execute<F, T>(&mut self, operation: F) -> Result<T, EditError>
    where
        F: FnOnce(&mut Self) -> Result<T, EditError>,
    {
        // 現在の状態を保存
        let saved_cursor = self.cursor;

        match operation(self) {
            Ok(result) => Ok(result),
            Err(error) => {
                // エラー時は状態を復旧
                self.cursor = saved_cursor;
                self.recover_from_error(&error)?;
                Err(error)
            }
        }
    }
}
```

## 7. パフォーマンス要件

### 7.1 性能目標

| 操作 | 目標応答時間 | 測定方法 |
|------|-------------|----------|
| 文字挿入 | < 1ms | カーソル位置での単文字挿入 |
| 文字削除 | < 1ms | Backspace/Delete操作 |
| カーソル移動 | < 1ms | QA.mdの要件 |
| 改行挿入 | < 1ms | Enter キー処理 |

### 7.2 最適化戦略

```rust
/// パフォーマンス最適化
impl TextEditor {
    /// 高速文字挿入（カーソル位置最適化）
    fn fast_insert_char(&mut self, ch: char) -> Result<(), EditError> {
        // ギャップバッファのギャップをカーソル位置に移動
        self.sync_gap_with_cursor()?;

        // ギャップ位置での挿入（O(1)）
        self.buffer.insert(self.cursor.char_pos, ch)?;

        // カーソルを前進
        self.cursor.char_pos += 1;
        self.cursor.column += 1;

        Ok(())
    }

    /// バッチ挿入最適化
    fn insert_string_optimized(&mut self, s: &str) -> Result<(), EditError> {
        if s.is_empty() {
            return Ok(());
        }

        // 一度にギャップを移動
        self.sync_gap_with_cursor()?;

        // 文字列全体を挿入
        self.buffer.insert_str(self.cursor.char_pos, s)?;

        // カーソル位置を更新
        let char_count = s.chars().count();
        self.cursor.char_pos += char_count;

        // 行・列位置を効率的に更新
        self.update_cursor_after_insert(s);

        Ok(())
    }

    /// 挿入後のカーソル位置更新
    fn update_cursor_after_insert(&mut self, inserted: &str) {
        for ch in inserted.chars() {
            if ch == '\n' {
                self.cursor.line += 1;
                self.cursor.column = 0;
            } else {
                self.cursor.column += 1;
            }
        }
    }
}
```

## 8. UTF-8安全性保証

### 8.1 文字境界保証メカニズム

```rust
/// UTF-8安全性検証
impl TextEditor {
    /// 操作前の安全性チェック
    fn validate_utf8_operation(&self, pos: usize, operation: &str) -> Result<(), EditError> {
        // 1. 位置の境界チェック
        if pos > self.buffer.len_chars() {
            return Err(EditError::OutOfBounds(pos));
        }

        // 2. 挿入内容のUTF-8検証
        if !operation.is_ascii() && !self.is_valid_utf8(operation) {
            return Err(EditError::InvalidChar('\0'));
        }

        Ok(())
    }

    /// UTF-8妥当性検証
    fn is_valid_utf8(&self, s: &str) -> bool {
        s.chars().all(|c| c != char::REPLACEMENT_CHARACTER || s.contains('�'))
    }

    /// 文字境界での操作保証
    fn ensure_char_boundary_operation<F>(&mut self, pos: usize, operation: F) -> Result<(), EditError>
    where
        F: FnOnce(&mut Self, usize) -> Result<(), EditError>,
    {
        // ギャップバッファは内部で文字単位管理のため、
        // 文字位置指定は常に文字境界
        operation(self, pos)
    }
}
```

## 9. テスト仕様

### 9.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_char_insertion() {
        let mut editor = TextEditor::new();

        // 基本文字挿入
        assert!(editor.insert_char('a').is_ok());
        assert_eq!(editor.buffer.to_string(), "a");
        assert_eq!(editor.cursor.char_pos, 1);
    }

    #[test]
    fn test_utf8_char_insertion() {
        let mut editor = TextEditor::new();

        // 日本語文字挿入
        assert!(editor.insert_char('あ').is_ok());
        assert_eq!(editor.buffer.to_string(), "あ");
        assert_eq!(editor.cursor.char_pos, 1);
    }

    #[test]
    fn test_backspace_deletion() {
        let mut editor = TextEditor::from_str("hello");
        editor.cursor.char_pos = 5;

        let deleted = editor.delete_backward().unwrap();
        assert_eq!(deleted, 'o');
        assert_eq!(editor.buffer.to_string(), "hell");
        assert_eq!(editor.cursor.char_pos, 4);
    }

    #[test]
    fn test_newline_insertion() {
        let mut editor = TextEditor::from_str("line1");
        editor.cursor.char_pos = 5;

        assert!(editor.insert_newline().is_ok());
        assert_eq!(editor.buffer.to_string(), "line1\n");
        assert_eq!(editor.cursor.line, 1);
        assert_eq!(editor.cursor.column, 0);
    }
}
```

### 9.2 パフォーマンステスト

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_char_insertion_performance() {
        let mut editor = TextEditor::new();

        let start = Instant::now();
        for i in 0..1000 {
            editor.insert_char('a').unwrap();
        }
        let duration = start.elapsed();

        // 1000文字挿入が10ms未満で完了することを確認
        assert!(duration.as_millis() < 10);
    }

    #[test]
    fn test_cursor_movement_performance() {
        let mut editor = TextEditor::from_str("a".repeat(1000));

        let start = Instant::now();
        for _ in 0..100 {
            editor.cursor.move_forward(&editor.buffer.to_string());
        }
        let duration = start.elapsed();

        // 100回のカーソル移動が1ms未満で完了することを確認
        assert!(duration.as_micros() < 1000);
    }
}
```

## 10. 実装フェーズ

### Phase 1: 基本構造（1日）
1. `TextEditor`構造体の定義
2. 基本的な文字挿入・削除
3. カーソル位置管理

### Phase 2: 安全性・性能（1日）
1. UTF-8安全性の実装
2. エラーハンドリング
3. パフォーマンス最適化

### Phase 3: 統合・テスト（1日）
1. キーバインドとの統合
2. 変更通知システム
3. 包括的テスト

## 11. 制限事項

### MVPでの制約
- IME統合は対象外
- 複合文字（結合文字）の詳細対応は対象外
- 自動インデントは対象外
- アンドゥ・リドゥは設計のみ

### 将来拡張予定
- IME入力システム
- グラフェムクラスタ対応
- 高度な自動インデント
- アンドゥ・リドゥ実装

この設計により、MVPに必要な基本編集機能を安全かつ高性能に実装し、将来の機能拡張にも対応可能な基盤を構築する。