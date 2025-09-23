# UTF-8安全な編集操作仕様書

## 概要

本文書は、AltreテキストエディタにおけるUTF-8文字境界を考慮した安全な編集操作の詳細仕様を定義する。不正な文字境界での操作を防止し、常に有効なUTF-8文字列状態を維持することを目的とする。

## 設計原則

1. **文字境界保証**: すべての編集操作は文字境界でのみ実行
2. **エンコーディング整合性**: 常に有効なUTF-8文字列状態を維持
3. **エラー防止**: 不正な操作は実行前に検出・拒否
4. **パフォーマンス**: 安全性チェックによる性能劣化を最小限に抑制

## UTF-8文字境界の定義

### 文字境界の識別

```rust
/// UTF-8文字境界判定
pub trait UTF8Boundary {
    /// 指定バイト位置が文字境界かどうかを判定
    fn is_char_boundary(&self, byte_pos: usize) -> bool;

    /// 指定文字位置が有効かどうかを判定
    fn is_valid_char_position(&self, char_pos: usize) -> bool;

    /// 文字位置をバイト位置に変換
    fn char_to_byte_pos(&self, char_pos: usize) -> Result<usize, UTF8Error>;

    /// バイト位置を文字位置に変換
    fn byte_to_char_pos(&self, byte_pos: usize) -> Result<usize, UTF8Error>;
}
```

### 文字境界検証の実装

```rust
impl UTF8Boundary for GapBuffer {
    fn is_char_boundary(&self, byte_pos: usize) -> bool {
        let text = self.to_string();
        text.is_char_boundary(byte_pos)
    }

    fn is_valid_char_position(&self, char_pos: usize) -> bool {
        char_pos <= self.len_chars()
    }

    fn char_to_byte_pos(&self, char_pos: usize) -> Result<usize, UTF8Error> {
        if !self.is_valid_char_position(char_pos) {
            return Err(UTF8Error::InvalidCharPosition(char_pos));
        }

        let text = self.to_string();
        let char_indices: Vec<_> = text.char_indices().collect();

        if char_pos == char_indices.len() {
            Ok(text.len())
        } else {
            Ok(char_indices[char_pos].0)
        }
    }

    fn byte_to_char_pos(&self, byte_pos: usize) -> Result<usize, UTF8Error> {
        let text = self.to_string();

        if byte_pos > text.len() {
            return Err(UTF8Error::InvalidBytePosition(byte_pos));
        }

        if !text.is_char_boundary(byte_pos) {
            return Err(UTF8Error::NotCharBoundary(byte_pos));
        }

        let prefix = &text[0..byte_pos];
        Ok(prefix.chars().count())
    }
}
```

## 安全な編集操作

### 文字挿入の安全性保証

```rust
/// 安全な文字挿入
impl SafeEditOperations for TextEditor {
    fn safe_insert_char(&mut self, char_pos: usize, ch: char) -> Result<(), UTF8Error> {
        // 1. 文字位置の検証
        if !self.buffer.is_valid_char_position(char_pos) {
            return Err(UTF8Error::InvalidCharPosition(char_pos));
        }

        // 2. 挿入文字の検証
        if !self.is_valid_utf8_char(ch) {
            return Err(UTF8Error::InvalidChar(ch));
        }

        // 3. 安全な挿入実行
        self.buffer.insert(char_pos, ch)
            .map_err(|e| UTF8Error::BufferError(e.to_string()))?;

        // 4. UTF-8整合性の後検証
        self.validate_utf8_integrity()?;

        Ok(())
    }

    fn safe_insert_str(&mut self, char_pos: usize, s: &str) -> Result<(), UTF8Error> {
        // 1. 文字位置の検証
        if !self.buffer.is_valid_char_position(char_pos) {
            return Err(UTF8Error::InvalidCharPosition(char_pos));
        }

        // 2. 文字列のUTF-8検証
        if !self.is_valid_utf8_string(s) {
            return Err(UTF8Error::InvalidString(s.to_string()));
        }

        // 3. 安全な挿入実行
        self.buffer.insert_str(char_pos, s)
            .map_err(|e| UTF8Error::BufferError(e.to_string()))?;

        // 4. UTF-8整合性の後検証
        self.validate_utf8_integrity()?;

        Ok(())
    }
}
```

### 文字削除の安全性保証

```rust
impl SafeEditOperations for TextEditor {
    fn safe_delete_char(&mut self, char_pos: usize) -> Result<char, UTF8Error> {
        // 1. 文字位置の検証
        if !self.buffer.is_valid_char_position(char_pos) {
            return Err(UTF8Error::InvalidCharPosition(char_pos));
        }

        if char_pos >= self.buffer.len_chars() {
            return Err(UTF8Error::OutOfBounds(char_pos));
        }

        // 2. 削除対象文字の取得と検証
        let deleted_char = self.get_char_at_position(char_pos)?;

        // 3. 安全な削除実行
        self.buffer.delete(char_pos)
            .map_err(|e| UTF8Error::BufferError(e.to_string()))?;

        // 4. UTF-8整合性の後検証
        self.validate_utf8_integrity()?;

        Ok(deleted_char)
    }

    fn safe_delete_range(&mut self, start: usize, end: usize) -> Result<String, UTF8Error> {
        // 1. 範囲の検証
        if start > end {
            return Err(UTF8Error::InvalidRange(start, end));
        }

        if !self.buffer.is_valid_char_position(start) {
            return Err(UTF8Error::InvalidCharPosition(start));
        }

        if !self.buffer.is_valid_char_position(end) {
            return Err(UTF8Error::InvalidCharPosition(end));
        }

        // 2. 削除内容の事前取得
        let deleted_text = self.buffer.substring(start, end)
            .map_err(|e| UTF8Error::BufferError(e.to_string()))?;

        // 3. 安全な削除実行
        self.buffer.delete_range(start, end)
            .map_err(|e| UTF8Error::BufferError(e.to_string()))?;

        // 4. UTF-8整合性の後検証
        self.validate_utf8_integrity()?;

        Ok(deleted_text)
    }
}
```

## UTF-8検証アルゴリズム

### 文字単位の検証

```rust
impl TextEditor {
    /// 有効なUTF-8文字かどうかを検証
    fn is_valid_utf8_char(&self, ch: char) -> bool {
        // Rustのcharは常に有効なUnicodeコードポイント
        // 制御文字やサロゲートの除外は別途実装
        !self.is_forbidden_char(ch)
    }

    /// 禁止文字の判定
    fn is_forbidden_char(&self, ch: char) -> bool {
        match ch {
            // NULL文字
            '\u{0000}' => true,
            // 制御文字（タブと改行は除外）
            '\u{0001}'..='\u{0008}' => true,
            '\u{000B}'..='\u{000C}' => true,
            '\u{000E}'..='\u{001F}' => true,
            '\u{007F}' => true,
            // サロゲートペア（Rustでは通常発生しない）
            '\u{D800}'..='\u{DFFF}' => true,
            // プライベート使用領域（必要に応じて制限）
            '\u{E000}'..='\u{F8FF}' => false, // 現在は許可
            // BOM文字（ファイル先頭以外では禁止）
            '\u{FEFF}' => self.should_reject_bom(),
            _ => false,
        }
    }

    /// BOM文字の拒否判定
    fn should_reject_bom(&self) -> bool {
        // カーソルがファイル先頭でなければBOMを拒否
        self.cursor.char_pos != 0
    }
}
```

### 文字列単位の検証

```rust
impl TextEditor {
    /// 有効なUTF-8文字列かどうかを検証
    fn is_valid_utf8_string(&self, s: &str) -> bool {
        // 1. 基本的なUTF-8妥当性チェック
        if !s.is_ascii() && !self.is_well_formed_utf8(s) {
            return false;
        }

        // 2. 禁止文字の検出
        if s.chars().any(|ch| self.is_forbidden_char(ch)) {
            return false;
        }

        // 3. 制御シーケンスの検証
        if !self.is_valid_control_sequence(s) {
            return false;
        }

        true
    }

    /// UTF-8形式の妥当性検証
    fn is_well_formed_utf8(&self, s: &str) -> bool {
        // Rustの&strは常に有効なUTF-8だが、
        // 将来のバイト列操作への備えとして実装
        std::str::from_utf8(s.as_bytes()).is_ok()
    }

    /// 制御シーケンスの妥当性検証
    fn is_valid_control_sequence(&self, s: &str) -> bool {
        // ANSI制御シーケンスなどの検証
        // MVPでは基本的な改行コードのみ考慮
        !s.contains('\r') || self.normalize_line_ending(s) == s
    }
}
```

## エラー型定義

### UTF-8エラー分類

```rust
/// UTF-8関連エラー
#[derive(Debug, thiserror::Error)]
pub enum UTF8Error {
    #[error("Invalid character position: {0}")]
    InvalidCharPosition(usize),

    #[error("Invalid byte position: {0}")]
    InvalidBytePosition(usize),

    #[error("Position {0} is not on a character boundary")]
    NotCharBoundary(usize),

    #[error("Invalid character: {0:?}")]
    InvalidChar(char),

    #[error("Invalid UTF-8 string: {0}")]
    InvalidString(String),

    #[error("Invalid range: start={0}, end={1}")]
    InvalidRange(usize, usize),

    #[error("Position {0} is out of bounds")]
    OutOfBounds(usize),

    #[error("Buffer operation failed: {0}")]
    BufferError(String),

    #[error("UTF-8 integrity check failed")]
    IntegrityCheckFailed,

    #[error("Encoding conversion failed")]
    ConversionFailed,
}
```

### エラー回復戦略

```rust
impl TextEditor {
    /// UTF-8エラーからの回復処理
    fn recover_from_utf8_error(&mut self, error: &UTF8Error) -> Result<(), UTF8Error> {
        match error {
            UTF8Error::InvalidCharPosition(_) |
            UTF8Error::NotCharBoundary(_) |
            UTF8Error::OutOfBounds(_) => {
                // カーソル位置を安全な位置に修正
                self.clamp_cursor_to_valid_position();
                Ok(())
            }
            UTF8Error::IntegrityCheckFailed => {
                // バッファの整合性を回復
                self.rebuild_buffer_consistency()?;
                Ok(())
            }
            UTF8Error::ConversionFailed => {
                // 変換失敗は致命的エラー
                Err(UTF8Error::ConversionFailed)
            }
            _ => Ok(())
        }
    }

    /// カーソルを有効な位置にクランプ
    fn clamp_cursor_to_valid_position(&mut self) {
        let max_pos = self.buffer.len_chars();
        if self.cursor.char_pos > max_pos {
            self.cursor.char_pos = max_pos;
        }

        // 文字境界でない場合は最寄りの境界に移動
        while self.cursor.char_pos > 0 {
            if self.buffer.is_valid_char_position(self.cursor.char_pos) {
                break;
            }
            self.cursor.char_pos -= 1;
        }
    }

    /// バッファ整合性の再構築
    fn rebuild_buffer_consistency(&mut self) -> Result<(), UTF8Error> {
        // 現在のバッファ内容を取得
        let current_text = self.buffer.to_string();

        // UTF-8妥当性を検証
        if !self.is_valid_utf8_string(&current_text) {
            return Err(UTF8Error::IntegrityCheckFailed);
        }

        // カーソル位置の再同期
        self.sync_cursor_with_buffer();

        Ok(())
    }
}
```

## パフォーマンス最適化

### 文字境界キャッシング

```rust
/// 文字境界キャッシュ
#[derive(Debug, Clone)]
pub struct CharBoundaryCache {
    /// 最後にアクセスした文字位置
    last_char_pos: usize,
    /// 対応するバイト位置
    last_byte_pos: usize,
    /// 文字インデックスのキャッシュ
    char_indices: Vec<(usize, char)>,
    /// キャッシュの有効性
    is_valid: bool,
}

impl CharBoundaryCache {
    pub fn new() -> Self {
        Self {
            last_char_pos: 0,
            last_byte_pos: 0,
            char_indices: Vec::new(),
            is_valid: false,
        }
    }

    /// キャッシュの無効化
    pub fn invalidate(&mut self) {
        self.is_valid = false;
        self.char_indices.clear();
    }

    /// 効率的な文字位置→バイト位置変換
    pub fn char_to_byte_cached(&mut self, text: &str, char_pos: usize) -> Result<usize, UTF8Error> {
        if !self.is_valid {
            self.rebuild_cache(text);
        }

        // 範囲チェック
        if char_pos > self.char_indices.len() {
            return Err(UTF8Error::InvalidCharPosition(char_pos));
        }

        // 末尾位置の場合
        if char_pos == self.char_indices.len() {
            return Ok(text.len());
        }

        // キャッシュから取得
        Ok(self.char_indices[char_pos].0)
    }

    /// キャッシュの再構築
    fn rebuild_cache(&mut self, text: &str) {
        self.char_indices = text.char_indices().collect();
        self.is_valid = true;
    }
}
```

### 高速検証アルゴリズム

```rust
impl TextEditor {
    /// 高速UTF-8整合性チェック
    fn fast_utf8_integrity_check(&self) -> Result<(), UTF8Error> {
        let text = self.buffer.to_string();

        // 1. 基本長さチェック
        if text.len() != self.buffer.len_bytes() {
            return Err(UTF8Error::IntegrityCheckFailed);
        }

        // 2. ASCII範囲の高速チェック
        if text.is_ascii() {
            return Ok(());
        }

        // 3. UTF-8バイトシーケンスの検証
        for ch in text.chars() {
            if !self.is_valid_utf8_char(ch) {
                return Err(UTF8Error::InvalidChar(ch));
            }
        }

        Ok(())
    }

    /// インクリメンタル検証（変更差分のみ）
    fn incremental_utf8_check(&self, start: usize, end: usize) -> Result<(), UTF8Error> {
        let text = self.buffer.to_string();

        // 変更範囲の前後を含めた安全な範囲を計算
        let safe_start = self.find_char_boundary_before(text.as_bytes(), start);
        let safe_end = self.find_char_boundary_after(text.as_bytes(), end);

        // 安全な範囲内でのUTF-8検証
        let segment = &text[safe_start..safe_end];
        if !self.is_valid_utf8_string(segment) {
            return Err(UTF8Error::IntegrityCheckFailed);
        }

        Ok(())
    }

    /// 指定位置前の文字境界を検索
    fn find_char_boundary_before(&self, bytes: &[u8], pos: usize) -> usize {
        let mut check_pos = pos;
        while check_pos > 0 && !std::str::from_utf8(&bytes[0..=check_pos]).is_ok() {
            check_pos -= 1;
        }
        check_pos
    }

    /// 指定位置後の文字境界を検索
    fn find_char_boundary_after(&self, bytes: &[u8], pos: usize) -> usize {
        let mut check_pos = pos;
        while check_pos < bytes.len() && !std::str::from_utf8(&bytes[check_pos..]).is_ok() {
            check_pos += 1;
        }
        check_pos
    }
}
```

## 複合文字対応準備

### グラフェムクラスタ認識（将来実装）

```rust
/// 将来の複合文字対応インターフェース
pub trait GraphemeAware {
    /// グラフェム境界の判定
    fn is_grapheme_boundary(&self, char_pos: usize) -> bool;

    /// グラフェムクラスタ単位での操作
    fn delete_grapheme_backward(&mut self) -> Result<String, UTF8Error>;
    fn delete_grapheme_forward(&mut self) -> Result<String, UTF8Error>;

    /// グラフェム数の取得
    fn len_graphemes(&self) -> usize;
}

// MVPでは基本的なUTF-8文字単位の操作のみ実装
// unicode-segmentationクレートを使用した実装は将来バージョンで対応
```

## テスト仕様

### UTF-8安全性テスト

```rust
#[cfg(test)]
mod utf8_safety_tests {
    use super::*;

    #[test]
    fn test_valid_utf8_insertion() {
        let mut editor = TextEditor::new();

        // ASCII文字
        assert!(editor.safe_insert_char(0, 'a').is_ok());

        // 日本語文字
        assert!(editor.safe_insert_char(1, 'あ').is_ok());

        // 絵文字
        assert!(editor.safe_insert_char(2, '🚀').is_ok());

        assert_eq!(editor.to_string(), "aあ🚀");
    }

    #[test]
    fn test_invalid_char_rejection() {
        let mut editor = TextEditor::new();

        // NULL文字の拒否
        assert!(editor.safe_insert_char(0, '\u{0000}').is_err());

        // 制御文字の拒否
        assert!(editor.safe_insert_char(0, '\u{0001}').is_err());

        // DEL文字の拒否
        assert!(editor.safe_insert_char(0, '\u{007F}').is_err());
    }

    #[test]
    fn test_char_boundary_validation() {
        let mut editor = TextEditor::from_str("あいう");

        // 有効な文字位置
        assert!(editor.buffer.is_valid_char_position(0));
        assert!(editor.buffer.is_valid_char_position(1));
        assert!(editor.buffer.is_valid_char_position(2));
        assert!(editor.buffer.is_valid_char_position(3));

        // 無効な文字位置
        assert!(!editor.buffer.is_valid_char_position(4));
    }

    #[test]
    fn test_safe_deletion() {
        let mut editor = TextEditor::from_str("hello あいう world");

        // 日本語文字の削除
        assert!(editor.safe_delete_char(6).is_ok()); // 'あ'
        assert_eq!(editor.to_string(), "hello いう world");

        // 範囲削除
        assert!(editor.safe_delete_range(6, 8).is_ok()); // 'いう'
        assert_eq!(editor.to_string(), "hello  world");
    }

    #[test]
    fn test_utf8_integrity_check() {
        let mut editor = TextEditor::from_str("正常なUTF-8文字列");

        // 整合性チェック成功
        assert!(editor.fast_utf8_integrity_check().is_ok());

        // 正常な操作後も整合性維持
        editor.safe_insert_char(0, '🎉').unwrap();
        assert!(editor.fast_utf8_integrity_check().is_ok());
    }

    #[test]
    fn test_line_ending_normalization() {
        let mut editor = TextEditor::new();

        // Windows CRLF
        let windows_text = "line1\r\nline2\r\n";
        assert!(editor.safe_insert_str(0, windows_text).is_ok());
        assert_eq!(editor.to_string(), "line1\nline2\n");

        // Mac CR
        let mut editor2 = TextEditor::new();
        let mac_text = "line1\rline2\r";
        assert!(editor2.safe_insert_str(0, mac_text).is_ok());
        assert_eq!(editor2.to_string(), "line1\nline2\n");
    }

    #[test]
    fn test_boundary_error_recovery() {
        let mut editor = TextEditor::from_str("test");

        // 無効な位置での操作
        assert!(editor.safe_delete_char(10).is_err());

        // エラー後もエディタ状態は正常
        assert!(editor.safe_insert_char(4, '!').is_ok());
        assert_eq!(editor.to_string(), "test!");
    }
}
```

### パフォーマンステスト

```rust
#[cfg(test)]
mod utf8_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_large_text_utf8_validation() {
        // 大きなUTF-8テキストでの性能テスト
        let large_text = "あ".repeat(10000);
        let mut editor = TextEditor::new();

        let start = Instant::now();
        assert!(editor.safe_insert_str(0, &large_text).is_ok());
        let duration = start.elapsed();

        // 10000文字の挿入が100ms未満で完了することを確認
        assert!(duration.as_millis() < 100);
    }

    #[test]
    fn test_char_boundary_cache_performance() {
        let mut cache = CharBoundaryCache::new();
        let text = "a".repeat(1000) + &"あ".repeat(1000);

        let start = Instant::now();
        for i in 0..1000 {
            cache.char_to_byte_cached(&text, i).unwrap();
        }
        let duration = start.elapsed();

        // キャッシュありで1000回の変換が10ms未満で完了
        assert!(duration.as_millis() < 10);
    }
}
```

## 実装ガイドライン

### 開発フェーズ

1. **Phase 1: 基本検証機能**
   - 文字境界判定
   - 基本的なUTF-8検証
   - エラー型定義

2. **Phase 2: 安全な操作**
   - 安全な挿入・削除操作
   - エラー回復機能
   - 整合性チェック

3. **Phase 3: 最適化**
   - 文字境界キャッシュ
   - インクリメンタル検証
   - パフォーマンス改善

### 実装注意事項

1. **Rustの安全性活用**: Rustの`&str`型は常に有効なUTF-8であることを活用
2. **文字単位操作**: バイト単位操作を避け、常に文字単位で操作
3. **検証の最適化**: 不要な検証を避け、必要最小限のチェックを実装
4. **エラー処理**: 回復可能なエラーと致命的エラーを明確に分離

この仕様により、UTF-8テキストの安全で効率的な編集を実現し、文字化けや不正な文字境界での操作を防止する。