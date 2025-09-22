# ギャップバッファ プロパティベーステスト仕様

## 概要

本文書は、ギャップバッファの実装における不変条件とプロパティを定義し、proptestクレートを使用したプロパティベーステストの仕様を策定する。

## 不変条件（Invariants）

### 1. バッファ構造の整合性

**条件**: ギャップの境界が常に有効
```rust
gap_start <= gap_end <= buffer.len()
```

**テスト戦略**:
- 任意の操作実行後にこの条件を検証
- バッファサイズ変更時の境界チェック

### 2. UTF-8エンコーディングの正当性

**条件**: ギャップを除いたバッファ内容が常に有効なUTF-8
```rust
std::str::from_utf8(&prefix_data).is_ok() &&
std::str::from_utf8(&suffix_data).is_ok()
```

**テスト戦略**:
- 任意のUTF-8文字列から開始
- 挿入・削除操作後のUTF-8妥当性検証
- マルチバイト文字の境界での操作テスト

### 3. データ保持の一貫性

**条件**: 論理的なテキスト内容が操作前後で期待通りに変化
```rust
// 挿入前後での文字数の変化
old_len + inserted_chars_count == new_len

// 削除前後での文字数の変化
old_len - deleted_chars_count == new_len
```

**テスト戦略**:
- 単純な文字列実装との結果比較
- 操作前後での文字数・バイト数の検証

## プロパティテスト仕様

### 1. 基本操作のプロパティ

#### 1.1 挿入操作の可換性
```rust
// Property: Insert operations at different positions are independent
forall text: String, pos1: usize, pos2: usize, char1: char, char2: char where pos1 != pos2 {
    let mut buf1 = GapBuffer::from_str(&text);
    let mut buf2 = GapBuffer::from_str(&text);

    // 異なる順序で挿入
    buf1.insert(pos1, char1);
    buf1.insert(pos2 + if pos1 <= pos2 { 1 } else { 0 }, char2);

    buf2.insert(pos2, char2);
    buf2.insert(pos1 + if pos2 <= pos1 { 1 } else { 0 }, char1);

    // 結果が同じであることを検証
    assert_eq!(buf1.to_string(), buf2.to_string());
}
```

#### 1.2 削除操作の妥当性
```rust
// Property: Delete operation removes exactly one character
forall text: String, pos: usize where !text.is_empty() && pos < text.chars().count() {
    let original_chars: Vec<char> = text.chars().collect();
    let mut buffer = GapBuffer::from_str(&text);

    let deleted_char = buffer.delete(pos).unwrap();
    let result_chars: Vec<char> = buffer.to_string().chars().collect();

    // 削除された文字が期待通り
    assert_eq!(deleted_char, original_chars[pos]);

    // 文字数が1減っている
    assert_eq!(result_chars.len(), original_chars.len() - 1);

    // 他の文字は保持されている
    for (i, &ch) in result_chars.iter().enumerate() {
        let original_index = if i < pos { i } else { i + 1 };
        assert_eq!(ch, original_chars[original_index]);
    }
}
```

#### 1.3 挿入・削除の逆操作
```rust
// Property: Insert and delete are inverse operations
forall text: String, pos: usize, ch: char where pos <= text.chars().count() {
    let mut buffer = GapBuffer::from_str(&text);

    buffer.insert(pos, ch).unwrap();
    let deleted = buffer.delete(pos).unwrap();

    // 元のテキストに戻る
    assert_eq!(buffer.to_string(), text);
    assert_eq!(deleted, ch);
}
```

### 2. UTF-8安全性のプロパティ

#### 2.1 マルチバイト文字の挿入
```rust
// Property: Multi-byte characters are handled correctly
forall base_text: String, pos: usize, unicode_char: char
where pos <= base_text.chars().count() && unicode_char.len_utf8() > 1 {
    let mut buffer = GapBuffer::from_str(&base_text);
    buffer.insert(pos, unicode_char).unwrap();

    let result = buffer.to_string();
    let result_chars: Vec<char> = result.chars().collect();

    // 挿入された文字が正しい位置にある
    assert_eq!(result_chars[pos], unicode_char);

    // UTF-8として有効
    assert!(result.is_ascii() || std::str::from_utf8(result.as_bytes()).is_ok());
}
```

#### 2.2 文字境界での操作安全性
```rust
// Property: Operations respect character boundaries
forall text: String, char_pos: usize
where !text.is_empty() && char_pos < text.chars().count() {
    let buffer = GapBuffer::from_str(&text);

    // substring操作が文字境界を尊重する
    for end_pos in char_pos..=text.chars().count() {
        let substring = buffer.substring(char_pos, end_pos).unwrap();
        let expected: String = text.chars().skip(char_pos).take(end_pos - char_pos).collect();
        assert_eq!(substring, expected);
    }
}
```

### 3. パフォーマンス特性のプロパティ

#### 3.1 ギャップ位置での効率性
```rust
// Property: Operations at gap position are more efficient
// これは実際のベンチマークテストで検証
forall text: String, operations: Vec<(usize, char)> {
    let mut buffer = GapBuffer::from_str(&text);

    // ギャップ位置での連続操作
    let gap_pos = buffer.gap_position();
    for (_, ch) in &operations {
        buffer.insert(gap_pos, *ch).unwrap();
    }

    // 結果の整合性のみ検証（時間計測は別途）
    let result = buffer.to_string();
    assert!(std::str::from_utf8(result.as_bytes()).is_ok());
}
```

#### 3.2 メモリ使用量の妥当性
```rust
// Property: Memory usage is reasonable
forall text: String where text.len() < 1_000_000 { // 1MB制限
    let buffer = GapBuffer::from_str(&text);

    // メモリ使用量がファイルサイズの合理的な倍数以内
    let text_size = text.as_bytes().len();
    let buffer_capacity = buffer.buffer.capacity();

    // 最大でもファイルサイズの3倍以内（ギャップ + オーバーヘッド）
    assert!(buffer_capacity <= text_size * 3 + 4096);

    // 最低でもファイルサイズ以上
    assert!(buffer_capacity >= text_size);
}
```

### 4. エラーハンドリングのプロパティ

#### 4.1 境界外アクセスの検出
```rust
// Property: Out-of-bounds operations are properly rejected
forall text: String, invalid_pos: usize
where invalid_pos > text.chars().count() {
    let mut buffer = GapBuffer::from_str(&text);

    // 範囲外の位置での操作は失敗する
    assert!(buffer.insert(invalid_pos, 'x').is_err());
    if !text.is_empty() {
        assert!(buffer.delete(invalid_pos).is_err());
    }

    // バッファの状態は変更されない
    assert_eq!(buffer.to_string(), text);
}
```

#### 4.2 不正なUTF-8シーケンスの拒否
```rust
// Property: Invalid UTF-8 sequences are rejected
// 実装では内部的にUTF-8を保証しているため、
// 外部からの不正入力に対する防御をテスト
forall text: String {
    let buffer = GapBuffer::from_str(&text);

    // 取得したテキストは常に有効なUTF-8
    let result = buffer.to_string();
    assert!(std::str::from_utf8(result.as_bytes()).is_ok());
}
```

## テスト実装例

### Cargo.toml設定
```toml
[dev-dependencies]
proptest = "1.0"
```

### テストコード構造
```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_insert_delete_inverse(
            text in ".*",
            pos in 0usize..100,
            ch in any::<char>()
        ) {
            // 実装...
        }

        #[test]
        fn test_utf8_safety(
            text in ".*",
            unicode_chars in prop::collection::vec(any::<char>(), 0..50)
        ) {
            // 実装...
        }

        #[test]
        fn test_gap_movement_correctness(
            text in ".*",
            positions in prop::collection::vec(0usize..100, 0..20)
        ) {
            // 実装...
        }
    }
}
```

## テスト戦略

### 1. データ生成戦略
- **テキスト生成**: ASCII、マルチバイト文字、混在パターン
- **位置生成**: 有効範囲、境界値、無効値
- **操作シーケンス**: ランダムな挿入・削除の組み合わせ

### 2. 実行設定
- **ケース数**: 各プロパティで1000-10000ケース
- **サイズ制限**: テキストサイズ1MB以下
- **時間制限**: テストスイート全体で5分以内

### 3. 回帰テスト
- **失敗ケースの保存**: proptestのシュリンキング機能活用
- **最小再現ケース**: 失敗時の最小例をユニットテストとして保存

## パフォーマンステスト

### ベンチマーク対象
1. **挿入性能**: ギャップ位置 vs 任意位置
2. **削除性能**: 単一文字 vs 範囲削除
3. **ギャップ移動**: 距離別の性能特性
4. **メモリ効率**: ファイルサイズ別の使用量

### 性能目標（QA.mdの回答に基づく）
- **カーソル移動**: < 1ms
- **基本的な編集操作**: < 10ms（大きなファイル）
- **メモリ使用量**: システムメモリの動的制限内

## 継続的テスト

### CI/CD統合
- **プルリクエスト**: 全プロパティテスト実行
- **夜間ビルド**: 大規模データでの長時間テスト
- **パフォーマンス回帰**: ベンチマーク結果の監視

### テストメトリクス
- **カバレッジ**: 全APIの実行パスカバー
- **エラー検出率**: 既知の不具合パターンの検出
- **実行時間**: テストスイートの実行時間監視