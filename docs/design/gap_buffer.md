# ギャップバッファ設計仕様書

## 概要

本文書は、Altreテキストエディタで使用するギャップバッファの詳細設計仕様を定義する。ギャップバッファは効率的なテキスト編集操作を実現するためのデータ構造であり、特にカーソル位置での挿入・削除操作において高いパフォーマンスを提供する。

## 設計目標

1. **効率性**: カーソル位置での挿入・削除操作をO(1)で実現
2. **安全性**: UTF-8文字境界を考慮した安全な操作保証
3. **拡張性**: 将来のロープ/ピーステーブルへの移行を考慮した抽象化
4. **メモリ効率**: 適切なメモリ使用量の管理

## データ構造設計

### 基本構造

```rust
pub struct GapBuffer {
    // テキストデータを格納するバッファ
    buffer: Vec<u8>,

    // ギャップの開始位置（バイト単位）
    gap_start: usize,

    // ギャップの終了位置（バイト単位）
    gap_end: usize,

    // 文字境界のキャッシュ（最適化）
    char_cache: Option<CharBoundaryCache>,
}
```

### メモリレイアウト

```
[prefix_text][gap_space][suffix_text]
              ↑        ↑
         gap_start  gap_end
```

- **prefix_text**: ギャップより前のテキストデータ
- **gap_space**: 未使用領域（新しい文字の挿入用）
- **suffix_text**: ギャップより後のテキストデータ

### ギャップサイズ戦略

#### 初期値
- **デフォルト**: 4KB（4096バイト）
- **理由**: メモリ効率と挿入性能のバランス

#### 拡張戦略
1. **最小拡張**: 現在のギャップサイズの2倍
2. **最大拡張**: 64KB上限
3. **縮小**: ギャップサイズがファイルサイズの50%を超える場合に実行

## API設計

### 基本操作

```rust
impl GapBuffer {
    /// 新しいギャップバッファを作成
    pub fn new() -> Self;

    /// 初期容量を指定してギャップバッファを作成
    pub fn with_capacity(capacity: usize) -> Self;

    /// 文字列からギャップバッファを作成
    pub fn from_str(s: &str) -> Self;

    /// 指定位置に文字を挿入
    pub fn insert(&mut self, pos: usize, ch: char) -> Result<(), BufferError>;

    /// 指定位置に文字列を挿入
    pub fn insert_str(&mut self, pos: usize, s: &str) -> Result<(), BufferError>;

    /// 指定位置の文字を削除
    pub fn delete(&mut self, pos: usize) -> Result<char, BufferError>;

    /// 指定範囲を削除
    pub fn delete_range(&mut self, start: usize, end: usize) -> Result<String, BufferError>;

    /// 指定範囲を置換
    pub fn replace_range(&mut self, start: usize, end: usize, s: &str) -> Result<(), BufferError>;

    /// 全体を文字列として取得
    pub fn to_string(&self) -> String;

    /// 指定範囲を文字列として取得
    pub fn substring(&self, start: usize, end: usize) -> Result<String, BufferError>;

    /// 文字数を取得
    pub fn len_chars(&self) -> usize;

    /// バイト数を取得
    pub fn len_bytes(&self) -> usize;

    /// 空かどうかを判定
    pub fn is_empty(&self) -> bool;
}
```

### カーソル操作

```rust
impl GapBuffer {
    /// ギャップ（カーソル）を指定位置に移動
    pub fn move_gap_to(&mut self, pos: usize) -> Result<(), BufferError>;

    /// 現在のギャップ位置を取得
    pub fn gap_position(&self) -> usize;
}
```

### イテレータ

```rust
impl GapBuffer {
    /// 文字イテレータを取得
    pub fn chars(&self) -> CharIterator;

    /// 行イテレータを取得
    pub fn lines(&self) -> LineIterator;
}
```

## UTF-8サポート

### 文字境界の保証

1. **挿入位置検証**: 文字境界でない位置での操作は`BufferError::InvalidPosition`を返す
2. **範囲検証**: 範囲指定時も文字境界を検証
3. **安全な変換**: バイト位置から文字位置への変換をサポート

### 文字境界キャッシュ

```rust
struct CharBoundaryCache {
    // 最後にアクセスした位置
    last_char_pos: usize,
    last_byte_pos: usize,

    // 行の開始位置のキャッシュ
    line_starts: Vec<usize>,
}
```

## エラーハンドリング

```rust
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Position {0} is out of bounds")]
    OutOfBounds(usize),

    #[error("Position {0} is not on a character boundary")]
    InvalidPosition(usize),

    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,

    #[error("Memory allocation failed")]
    AllocationFailed,
}
```

## パフォーマンス特性

### 計算量

| 操作 | 最良 | 平均 | 最悪 |
|------|------|------|------|
| insert (ギャップ位置) | O(1) | O(1) | O(1) |
| insert (任意位置) | O(1) | O(n) | O(n) |
| delete (ギャップ位置) | O(1) | O(1) | O(1) |
| delete (任意位置) | O(1) | O(n) | O(n) |
| move_gap_to | O(1) | O(n) | O(n) |
| substring | O(k) | O(k) | O(k) |

※ nはバッファサイズ、kは取得する文字列の長さ

### メモリ使用量

- **ベースメモリ**: ファイルサイズ + ギャップサイズ
- **追加メモリ**: 文字境界キャッシュ（約ファイルサイズの1-5%）

### 性能目標

- **挿入・削除（ギャップ位置）**: < 1μs
- **ギャップ移動（1KB）**: < 10μs
- **文字列変換（1MB）**: < 10ms

## 実装考慮事項

### Rustの所有権システム

1. **借用チェッカー対応**: 可変参照と不変参照の適切な分離
2. **ライフタイム管理**: イテレータのライフタイム設計
3. **ゼロコスト抽象化**: 不要なアロケーションの回避

### 最適化戦略

1. **ギャップ位置の局所性**: 連続した操作でのギャップ移動最小化
2. **文字境界キャッシュ**: 頻繁なアクセスパターンの最適化
3. **バッファ拡張**: 指数的拡張による再アロケーション削減

### テスト戦略

1. **ユニットテスト**: 基本操作の正確性
2. **プロパティテスト**: 不変条件の検証
3. **ベンチマークテスト**: パフォーマンス回帰の検出

## 将来の拡張

### ロープ/ピーステーブルへの移行

```rust
pub trait TextBuffer {
    type Error;

    fn insert(&mut self, pos: usize, s: &str) -> Result<(), Self::Error>;
    fn delete_range(&mut self, start: usize, end: usize) -> Result<String, Self::Error>;
    fn substring(&self, start: usize, end: usize) -> Result<String, Self::Error>;
    // ...
}

impl TextBuffer for GapBuffer {
    // 実装...
}
```

### 大きなファイル対応

- **遅延読み込み**: ファイルの一部のみをメモリに保持
- **ページング**: メモリマッピングの活用
- **圧縮**: 未使用領域の圧縮

## 制限事項

1. **ファイルサイズ**: 推奨上限100MB（システムメモリに依存）
2. **文字エンコーディング**: UTF-8のみサポート
3. **並行性**: シングルスレッドでの使用を前提

## 参考文献

- "Crafting Interpreters" - Text Editors and Gap Buffers
- Emacs実装におけるギャップバッファの使用例
- VSCode/Monaco Editorのテキストバッファ設計