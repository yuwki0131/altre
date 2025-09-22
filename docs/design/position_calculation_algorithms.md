# 位置計算アルゴリズム仕様書

## 概要

本文書は、Altreテキストエディタのナビゲーションシステムで使用される位置計算アルゴリズムの詳細仕様を定義する。文字位置、行・列位置、表示位置の相互変換を高速かつ正確に行うアルゴリズムを提供する。

## 設計目標

1. **高速性**: カーソル移動 < 1ms（QA要件）の実現
2. **正確性**: UTF-8文字境界での正確な位置計算
3. **効率性**: メモリ使用量の最適化
4. **拡張性**: 長い行（QA Q22: 段階的制限）への対応

## 座標系定義

### 基本座標系

```rust
/// 統合座標情報
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// 文字位置（UTF-8文字単位、0ベース）
    /// - ファイル先頭からの文字数
    /// - 改行文字も1文字としてカウント
    pub char_pos: usize,

    /// 行番号（0ベース）
    /// - ファイル先頭の行が0
    /// - 改行文字で行が区切られる
    pub line: usize,

    /// 表示列番号（Tab考慮、0ベース）
    /// - Tabの表示幅を考慮した画面上の位置
    /// - 全角文字の表示幅を考慮
    pub visual_column: usize,

    /// 論理列番号（文字数、0ベース）
    /// - 行頭からの文字数
    /// - Tabも1文字としてカウント
    pub logical_column: usize,
}
```

### 座標系変換の例

```
テキスト: "a\tあ🌟"
Tab幅: 4

文字位置: 0  1  2  3
文字内容: a \t あ 🌟
論理列:   0  1  2  3
表示列:   0  4  6  8

説明:
- a: 論理列0, 表示列0
- \t: 論理列1, 表示列4 (Tab幅4で次の4の倍数位置)
- あ: 論理列2, 表示列6 (全角文字で幅2)
- 🌟: 論理列3, 表示列8 (絵文字で幅2)
```

## 1. 基本位置計算アルゴリズム

### 1.1 文字位置→行・列位置変換

```rust
/// 効率的な文字位置から行・列位置への変換
impl PositionCalculator {
    /// O(log n) 行検索 + O(k) 列計算（nは行数、kは行内文字数）
    pub fn char_pos_to_line_col(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        // 1. キャッシュの有効性確認
        if !self.cache_valid {
            self.rebuild_line_cache(text);
        }

        // 2. バイナリサーチで行を特定 O(log n)
        let line = self.binary_search_line(char_pos)?;

        // 3. 行内での位置計算 O(k)
        let line_start = self.line_index_cache[line];
        let logical_column = char_pos - line_start;

        // 4. 表示列の計算
        let line_text = self.get_line_text(text, line);
        let visual_column = self.calculate_visual_column(&line_text, logical_column)?;

        Ok(Position {
            char_pos,
            line,
            visual_column,
            logical_column,
        })
    }

    /// バイナリサーチによる高速行検索
    fn binary_search_line(&self, char_pos: usize) -> Result<usize, NavigationError> {
        match self.line_index_cache.binary_search(&char_pos) {
            // 正確に行の開始位置にある場合
            Ok(line) => Ok(line),
            // 行の途中にある場合
            Err(insertion_point) => {
                if insertion_point == 0 {
                    return Err(NavigationError::InvalidPosition(char_pos));
                }
                Ok(insertion_point - 1)
            }
        }
    }
}
```

### 1.2 行・列位置→文字位置変換

```rust
impl PositionCalculator {
    /// O(1) 行インデックス参照 + O(k) 列検証
    pub fn line_col_to_char_pos(&mut self, text: &str, line: usize, logical_column: usize) -> Result<usize, NavigationError> {
        // 1. キャッシュの確認
        if !self.cache_valid {
            self.rebuild_line_cache(text);
        }

        // 2. 行の有効性確認 O(1)
        if line >= self.line_index_cache.len() {
            return Err(NavigationError::InvalidLine(line));
        }

        // 3. 行の開始位置を取得 O(1)
        let line_start = self.line_index_cache[line];

        // 4. 列位置の有効性確認とクランプ O(k)
        let line_text = self.get_line_text(text, line);
        let line_length = line_text.chars().count();
        let clamped_column = logical_column.min(line_length);

        Ok(line_start + clamped_column)
    }
}
```

## 2. 行インデックスキャッシュシステム

### 2.1 キャッシュ構造

```rust
/// 高速アクセス用のキャッシュ構造
pub struct PositionCalculator {
    /// 行の開始位置のインデックス
    /// line_index_cache[i] = i行目の開始文字位置
    line_index_cache: Vec<usize>,

    /// キャッシュの有効性フラグ
    cache_valid: bool,

    /// Tab幅設定（QA Q21: 4スペース）
    tab_width: usize,

    /// 長い行用最適化フラグ
    long_line_optimization: bool,

    /// 最大行長のキャッシュ（最適化判定用）
    max_line_length: Option<usize>,
}
```

### 2.2 キャッシュ構築アルゴリズム

```rust
impl PositionCalculator {
    /// O(n) 線形スキャンによるキャッシュ構築（nは文字数）
    fn rebuild_line_cache(&mut self, text: &str) {
        let start_time = std::time::Instant::now();

        self.line_index_cache.clear();
        self.line_index_cache.reserve(text.lines().count() + 1);

        // 最初の行は0から開始
        self.line_index_cache.push(0);

        let mut char_pos = 0;
        let mut max_line_length = 0;
        let mut current_line_length = 0;

        for ch in text.chars() {
            char_pos += 1;
            current_line_length += 1;

            if ch == '\n' {
                self.line_index_cache.push(char_pos);
                max_line_length = max_line_length.max(current_line_length - 1); // 改行文字を除く
                current_line_length = 0;
            }
        }

        // 最後の行の長さも考慮
        max_line_length = max_line_length.max(current_line_length);
        self.max_line_length = Some(max_line_length);

        self.cache_valid = true;

        // パフォーマンス監視
        let duration = start_time.elapsed();
        if duration.as_millis() > 10 {
            eprintln!("Warning: Line cache rebuild took {:?}", duration);
        }
    }

    /// インクリメンタルキャッシュ更新（編集操作用）
    pub fn update_cache_incremental(&mut self, text: &str, edit_start: usize, edit_end: usize, inserted_text: &str) {
        if !self.cache_valid {
            self.rebuild_line_cache(text);
            return;
        }

        // 影響を受ける行の範囲を特定
        let affected_start_line = self.binary_search_line(edit_start).unwrap_or(0);

        // 編集が改行文字に影響する場合は完全再構築
        let has_newline_changes = text[edit_start..edit_end].contains('\n') || inserted_text.contains('\n');

        if has_newline_changes {
            // 改行文字が関わる場合は完全再構築
            self.rebuild_line_cache(text);
        } else {
            // 同一行内の編集の場合はキャッシュは有効
            // (行の開始位置は変更されないため)
        }
    }
}
```

### 2.3 キャッシュ無効化戦略

```rust
impl PositionCalculator {
    /// キャッシュ無効化の条件
    pub fn invalidate_cache(&mut self) {
        self.cache_valid = false;
        self.max_line_length = None;
    }

    /// 部分的無効化（パフォーマンス最適化）
    pub fn invalidate_cache_from_line(&mut self, line: usize) {
        if line == 0 {
            // 先頭行からの変更は完全無効化
            self.invalidate_cache();
        } else {
            // 部分的無効化は複雑なため、現在は完全無効化
            // 将来の最適化でより細かい制御を実装
            self.invalidate_cache();
        }
    }
}
```

## 3. 表示列計算アルゴリズム

### 3.1 Tab幅考慮計算

```rust
impl Position {
    /// O(k) Tab考慮の表示列計算（kは対象列まで文字数）
    pub fn logical_to_visual_column(logical_col: usize, line_text: &str, tab_width: usize) -> usize {
        let mut visual_col = 0;
        let chars: Vec<char> = line_text.chars().collect();

        for i in 0..logical_col.min(chars.len()) {
            let ch = chars[i];

            if ch == '\t' {
                // 次のTab位置まで進む
                let next_tab_stop = ((visual_col / tab_width) + 1) * tab_width;
                visual_col = next_tab_stop;
            } else {
                // 文字の表示幅を加算
                visual_col += Self::char_display_width(ch);
            }
        }

        visual_col
    }

    /// 表示列から論理列への逆変換
    pub fn visual_to_logical_column(visual_col: usize, line_text: &str, tab_width: usize) -> usize {
        let mut current_visual = 0;
        let chars: Vec<char> = line_text.chars().collect();

        for (logical_pos, &ch) in chars.iter().enumerate() {
            if current_visual >= visual_col {
                return logical_pos;
            }

            if ch == '\t' {
                let next_tab_stop = ((current_visual / tab_width) + 1) * tab_width;
                current_visual = next_tab_stop;
            } else {
                current_visual += Self::char_display_width(ch);
            }

            if current_visual > visual_col {
                return logical_pos;
            }
        }

        chars.len()
    }
}
```

### 3.2 Unicode文字幅計算

```rust
impl Position {
    /// Unicode文字の表示幅計算（QA Q15: 基本対応）
    fn char_display_width(ch: char) -> usize {
        // 高速化のための分岐最適化
        if ch as u32 <= 0x7F {
            // ASCII範囲の高速パス
            return 1;
        }

        match ch {
            // 制御文字
            '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}' => 0,

            // 結合文字（将来実装）
            '\u{0300}'..='\u{036F}' |  // 結合記号
            '\u{1AB0}'..='\u{1AFF}' |  // 結合記号拡張
            '\u{1DC0}'..='\u{1DFF}' |  // 結合記号拡張
            '\u{20D0}'..='\u{20FF}' => 0, // 結合記号

            // 全角文字
            '\u{1100}'..='\u{115F}' |  // ハングル字母
            '\u{2E80}'..='\u{2EFF}' |  // CJK部首補助
            '\u{2F00}'..='\u{2FDF}' |  // 康熙部首
            '\u{3000}'..='\u{303F}' |  // CJK記号
            '\u{3040}'..='\u{309F}' |  // ひらがな
            '\u{30A0}'..='\u{30FF}' |  // カタカナ
            '\u{3100}'..='\u{312F}' |  // 注音字母
            '\u{3130}'..='\u{318F}' |  // ハングル互換字母
            '\u{3190}'..='\u{319F}' |  // 漢文用記号
            '\u{31A0}'..='\u{31BF}' |  // 注音拡張
            '\u{31C0}'..='\u{31EF}' |  // CJKストローク
            '\u{31F0}'..='\u{31FF}' |  // カタカナ拡張
            '\u{3200}'..='\u{32FF}' |  // CJK互換
            '\u{3300}'..='\u{33FF}' |  // CJK互換
            '\u{3400}'..='\u{4DBF}' |  // CJK拡張A
            '\u{4E00}'..='\u{9FFF}' |  // CJK統合漢字
            '\u{A000}'..='\u{A48F}' |  // イ語
            '\u{A490}'..='\u{A4CF}' |  // イ語部首
            '\u{AC00}'..='\u{D7AF}' |  // ハングル音節
            '\u{F900}'..='\u{FAFF}' |  // CJK互換漢字
            '\u{FE10}'..='\u{FE1F}' |  // 縦書き用記号
            '\u{FE30}'..='\u{FE4F}' |  // CJK互換形
            '\u{FE50}'..='\u{FE6F}' |  // 小字形
            '\u{FF00}'..='\u{FFEF}' => 2, // 全角英数・記号

            // 絵文字（基本）
            '\u{1F300}'..='\u{1F5FF}' |
            '\u{1F600}'..='\u{1F64F}' |
            '\u{1F680}'..='\u{1F6FF}' |
            '\u{1F700}'..='\u{1F77F}' |
            '\u{1F780}'..='\u{1F7FF}' |
            '\u{1F800}'..='\u{1F8FF}' |
            '\u{1F900}'..='\u{1F9FF}' |
            '\u{1FA00}'..='\u{1FA6F}' |
            '\u{1FA70}'..='\u{1FAFF}' => 2,

            // その他は1として扱う
            _ => 1,
        }
    }

    /// 文字幅計算のベンチマーク用関数
    #[cfg(test)]
    fn benchmark_char_width_calculation(text: &str) -> std::time::Duration {
        let start = std::time::Instant::now();

        for ch in text.chars() {
            let _ = Self::char_display_width(ch);
        }

        start.elapsed()
    }
}
```

## 4. 長い行対応アルゴリズム（QA Q22対応）

### 4.1 段階的最適化戦略

```rust
impl PositionCalculator {
    /// 行長に応じた最適化戦略の選択
    pub fn optimized_char_pos_to_line_col(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        let max_line_length = self.max_line_length.unwrap_or_else(|| self.estimate_max_line_length(text));

        match max_line_length {
            // 短い行（< 1000文字）: 通常アルゴリズム、< 1ms目標
            0..=999 => self.char_pos_to_line_col(text, char_pos),

            // 長い行（1000-10000文字）: 軽微な最適化、< 5ms許容
            1000..=9999 => self.optimized_calculation_medium_lines(text, char_pos),

            // 超長い行（> 10000文字）: 積極的最適化、< 10ms許容
            _ => self.optimized_calculation_long_lines(text, char_pos),
        }
    }

    /// 中程度の行用最適化
    fn optimized_calculation_medium_lines(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        // キャッシュの活用を強化
        if !self.cache_valid {
            self.rebuild_line_cache(text);
        }

        // 通常のアルゴリズムを使用（キャッシュ効果で十分高速）
        self.char_pos_to_line_col(text, char_pos)
    }

    /// 超長い行用最適化
    fn optimized_calculation_long_lines(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        // チャンク単位での処理により計算量を削減
        const CHUNK_SIZE: usize = 1000;

        let line = self.binary_search_line(char_pos)?;
        let line_start = self.line_index_cache[line];
        let relative_pos = char_pos - line_start;

        // 行内でのチャンク処理
        let line_text = self.get_line_text(text, line);
        let chars: Vec<char> = line_text.chars().collect();

        if chars.len() <= CHUNK_SIZE {
            // チャンクサイズ以下の場合は通常処理
            return self.char_pos_to_line_col(text, char_pos);
        }

        // チャンク単位で処理
        let chunk_index = relative_pos / CHUNK_SIZE;
        let chunk_start = chunk_index * CHUNK_SIZE;
        let chunk_end = (chunk_start + CHUNK_SIZE).min(chars.len());

        let logical_column = relative_pos;

        // 表示列の近似計算（精度よりも速度を重視）
        let visual_column = self.approximate_visual_column(&chars, logical_column)?;

        Ok(Position {
            char_pos,
            line,
            visual_column,
            logical_column,
        })
    }

    /// 表示列の近似計算（超長い行用）
    fn approximate_visual_column(&self, chars: &[char], logical_column: usize) -> Result<usize, NavigationError> {
        const SAMPLE_INTERVAL: usize = 100;

        if logical_column <= SAMPLE_INTERVAL {
            // 先頭付近は正確に計算
            return Ok(Position::logical_to_visual_column(logical_column, &chars.iter().collect::<String>(), self.tab_width));
        }

        // サンプリングによる近似
        let mut visual_col = 0;
        let mut i = 0;

        while i < logical_column {
            let next_sample = (i + SAMPLE_INTERVAL).min(logical_column);

            // サンプル区間での平均文字幅を推定
            let sample_chars = &chars[i..next_sample];
            let avg_width = self.estimate_average_char_width(sample_chars);

            visual_col += avg_width * (next_sample - i);
            i = next_sample;
        }

        Ok(visual_col)
    }

    /// 平均文字幅の推定
    fn estimate_average_char_width(&self, chars: &[char]) -> usize {
        if chars.is_empty() {
            return 1;
        }

        // 先頭数文字をサンプリング
        let sample_size = chars.len().min(10);
        let mut total_width = 0;

        for &ch in &chars[0..sample_size] {
            if ch == '\t' {
                total_width += self.tab_width;
            } else {
                total_width += Position::char_display_width(ch);
            }
        }

        (total_width / sample_size).max(1)
    }

    /// 最大行長の推定
    fn estimate_max_line_length(&self, text: &str) -> usize {
        // 効率的な推定：先頭数行をサンプリング
        text.lines()
            .take(100) // 先頭100行をサンプル
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
    }
}
```

## 5. パフォーマンス最適化技術

### 5.1 キャッシュ戦略

```rust
/// 位置計算のキャッシュシステム
pub struct PositionCache {
    /// 最近アクセスした位置のキャッシュ
    recent_positions: std::collections::LRUCache<usize, Position>,
    /// 行の統計情報キャッシュ
    line_stats: Vec<LineStatistics>,
}

#[derive(Debug, Clone)]
struct LineStatistics {
    /// 行の文字数
    char_count: usize,
    /// 行の表示幅
    visual_width: usize,
    /// Tab文字の数
    tab_count: usize,
    /// 全角文字の数
    wide_char_count: usize,
}

impl PositionCache {
    /// キャッシュを活用した高速位置計算
    pub fn cached_char_pos_to_line_col(&mut self, calc: &mut PositionCalculator, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        // LRUキャッシュから確認
        if let Some(cached_pos) = self.recent_positions.get(&char_pos) {
            return Ok(cached_pos.clone());
        }

        // キャッシュミスの場合は計算
        let position = calc.char_pos_to_line_col(text, char_pos)?;

        // 結果をキャッシュに保存
        self.recent_positions.put(char_pos, position.clone());

        Ok(position)
    }
}
```

### 5.2 SIMD最適化（将来実装）

```rust
#[cfg(target_arch = "x86_64")]
mod simd_optimizations {
    use std::arch::x86_64::*;

    /// SIMD命令を使用した高速文字数カウント
    /// 注意: 実装例のため、実際のMVPでは使用しない
    pub unsafe fn count_chars_simd(text: &[u8]) -> usize {
        // AVX2を使用したUTF-8文字数カウント
        // 実装は複雑なため、将来の最適化として残しておく
        text.len() // プレースホルダー
    }

    /// SIMD命令を使用した改行文字検索
    pub unsafe fn find_newlines_simd(text: &[u8]) -> Vec<usize> {
        // AVX2を使用した高速改行検索
        // 実装は複雑なため、将来の最適化として残しておく
        Vec::new() // プレースホルダー
    }
}
```

## 6. アルゴリズムの計算量分析

### 6.1 時間計算量

| 操作 | 最良ケース | 平均ケース | 最悪ケース | 備考 |
|------|------------|------------|------------|------|
| キャッシュ構築 | O(n) | O(n) | O(n) | nは文字数 |
| 文字位置→行・列 | O(log L + k) | O(log L + k) | O(log L + k) | Lは行数、kは行内文字数 |
| 行・列→文字位置 | O(1 + k) | O(1 + k) | O(1 + k) | k は列位置 |
| 表示列計算 | O(k) | O(k) | O(k) | kは論理列位置 |
| 長い行最適化 | O(log L + 1) | O(log L + s) | O(log L + s) | sはサンプルサイズ |

### 6.2 空間計算量

| データ構造 | 使用量 | 説明 |
|------------|---------|------|
| 行インデックスキャッシュ | O(L) × 8 bytes | Lは行数 |
| 位置キャッシュ | O(C) × 32 bytes | Cはキャッシュサイズ |
| 行統計キャッシュ | O(L) × 24 bytes | L行分の統計情報 |
| 総メモリ使用量 | O(L) × 64 bytes | 概算値 |

### 6.3 実測パフォーマンス目標

```rust
#[cfg(test)]
mod performance_targets {
    use super::*;

    /// パフォーマンス目標の検証
    #[test]
    fn verify_performance_targets() {
        let test_cases = vec![
            ("small", "a".repeat(100), Duration::from_micros(100)),
            ("medium", "a".repeat(1000), Duration::from_millis(1)),
            ("large", "a".repeat(10000), Duration::from_millis(5)),
            ("xlarge", "a".repeat(100000), Duration::from_millis(10)),
        ];

        for (name, text, target) in test_cases {
            let mut calc = PositionCalculator::new();

            let start = Instant::now();
            for i in (0..text.len()).step_by(100) {
                calc.char_pos_to_line_col(&text, i).unwrap();
            }
            let duration = start.elapsed();

            assert!(
                duration <= target,
                "{} case exceeded target: {:?} > {:?}",
                name, duration, target
            );
        }
    }
}
```

## 7. エラーハンドリングと境界条件

### 7.1 境界条件の処理

```rust
impl PositionCalculator {
    /// 境界条件での安全な位置計算
    pub fn safe_char_pos_to_line_col(&mut self, text: &str, char_pos: usize) -> Result<Position, NavigationError> {
        // 1. 入力値の検証
        let text_length = text.chars().count();
        if char_pos > text_length {
            return Err(NavigationError::InvalidPosition(char_pos));
        }

        // 2. 空文字列の処理
        if text.is_empty() {
            if char_pos == 0 {
                return Ok(Position {
                    char_pos: 0,
                    line: 0,
                    visual_column: 0,
                    logical_column: 0,
                });
            } else {
                return Err(NavigationError::InvalidPosition(char_pos));
            }
        }

        // 3. ファイル末尾の処理
        if char_pos == text_length {
            let last_line = self.count_total_lines(text).saturating_sub(1);
            let line_length = self.calculate_line_length(text, last_line);
            let visual_column = Position::logical_to_visual_column(line_length, &self.get_line_text(text, last_line), self.tab_width);

            return Ok(Position {
                char_pos,
                line: last_line,
                visual_column,
                logical_column: line_length,
            });
        }

        // 4. 通常の計算
        self.char_pos_to_line_col(text, char_pos)
    }

    /// 行数の計算
    fn count_total_lines(&self, text: &str) -> usize {
        if text.is_empty() {
            1
        } else {
            text.lines().count()
        }
    }
}
```

### 7.2 エラー回復アルゴリズム

```rust
impl PositionCalculator {
    /// 破損したキャッシュからの回復
    pub fn recover_from_cache_corruption(&mut self, text: &str) -> Result<(), NavigationError> {
        // 1. キャッシュの整合性確認
        if self.validate_cache_integrity(text) {
            return Ok(());
        }

        // 2. キャッシュの再構築
        self.invalidate_cache();
        self.rebuild_line_cache(text);

        // 3. 再構築後の検証
        if !self.validate_cache_integrity(text) {
            return Err(NavigationError::TextProcessingError(
                "Failed to rebuild position cache".to_string()
            ));
        }

        Ok(())
    }

    /// キャッシュ整合性の検証
    fn validate_cache_integrity(&self, text: &str) -> bool {
        if !self.cache_valid || self.line_index_cache.is_empty() {
            return false;
        }

        // 基本的な整合性チェック
        let expected_first_line = 0;
        if self.line_index_cache[0] != expected_first_line {
            return false;
        }

        let text_length = text.chars().count();
        let last_index = self.line_index_cache.last().copied().unwrap_or(0);
        if last_index > text_length {
            return false;
        }

        true
    }
}
```

## 8. テスト戦略

### 8.1 単体テスト

```rust
#[cfg(test)]
mod algorithm_tests {
    use super::*;

    #[test]
    fn test_basic_position_conversion() {
        let mut calc = PositionCalculator::new();
        let text = "Hello\nWorld\n";

        // 文字位置 6 = "World"の"W"
        let pos = calc.char_pos_to_line_col(text, 6).unwrap();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.logical_column, 0);

        // 逆変換
        let char_pos = calc.line_col_to_char_pos(text, 1, 0).unwrap();
        assert_eq!(char_pos, 6);
    }

    #[test]
    fn test_tab_width_calculation() {
        let mut calc = PositionCalculator::new();
        let text = "a\tb\tc";

        let pos = calc.char_pos_to_line_col(text, 2).unwrap(); // "b"の位置
        assert_eq!(pos.visual_column, 5); // a(1) + tab(4) = 5
    }

    #[test]
    fn test_unicode_characters() {
        let mut calc = PositionCalculator::new();
        let text = "aあ🌟b";

        let pos = calc.char_pos_to_line_col(text, 3).unwrap(); // "b"の位置
        assert_eq!(pos.logical_column, 3);
        assert_eq!(pos.visual_column, 5); // a(1) + あ(2) + 🌟(2) = 5
    }

    #[test]
    fn test_empty_text() {
        let mut calc = PositionCalculator::new();
        let text = "";

        let pos = calc.safe_char_pos_to_line_col(text, 0).unwrap();
        assert_eq!(pos.char_pos, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.logical_column, 0);
    }

    #[test]
    fn test_boundary_conditions() {
        let mut calc = PositionCalculator::new();
        let text = "single line";

        // ファイル末尾
        let text_length = text.chars().count();
        let pos = calc.safe_char_pos_to_line_col(text, text_length).unwrap();
        assert_eq!(pos.char_pos, text_length);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.logical_column, text_length);

        // 範囲外
        assert!(calc.safe_char_pos_to_line_col(text, text_length + 1).is_err());
    }
}
```

### 8.2 パフォーマンステスト

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_large_file_performance() {
        let large_text = "line\n".repeat(10000);
        let mut calc = PositionCalculator::new();

        let start = Instant::now();

        // 100回の位置計算
        for i in (0..large_text.chars().count()).step_by(large_text.chars().count() / 100) {
            calc.char_pos_to_line_col(&large_text, i).unwrap();
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 10, "Large file processing too slow: {:?}", duration);
    }

    #[test]
    fn test_cache_rebuild_performance() {
        let large_text = "a".repeat(100000);
        let mut calc = PositionCalculator::new();

        let start = Instant::now();
        calc.rebuild_line_cache(&large_text);
        let duration = start.elapsed();

        assert!(duration.as_millis() < 50, "Cache rebuild too slow: {:?}", duration);
    }

    #[test]
    fn test_long_line_performance() {
        let long_line = "a".repeat(50000);
        let mut calc = PositionCalculator::new();

        let start = Instant::now();
        calc.optimized_char_pos_to_line_col(&long_line, 25000).unwrap();
        let duration = start.elapsed();

        // QA Q22: 長い行では段階的制限、10ms許容
        assert!(duration.as_millis() < 10, "Long line processing too slow: {:?}", duration);
    }
}
```

## 9. 将来の拡張

### 9.1 インクリメンタル更新

```rust
/// 将来実装: 編集操作に対するインクリメンタル更新
impl PositionCalculator {
    /// 編集操作後の効率的なキャッシュ更新
    pub fn update_for_edit(
        &mut self,
        old_text: &str,
        new_text: &str,
        edit_start: usize,
        edit_end: usize,
    ) -> Result<(), NavigationError> {
        // 将来実装: 編集範囲のみの部分更新
        // 現在は完全再構築で対応
        self.rebuild_line_cache(new_text);
        Ok(())
    }
}
```

### 9.2 並列処理対応

```rust
/// 将来実装: マルチスレッド対応
impl PositionCalculator {
    /// 大きなファイルでの並列処理
    pub fn parallel_cache_rebuild(&mut self, text: &str) -> Result<(), NavigationError> {
        // 将来実装: チャンク単位での並列処理
        // 現在はシングルスレッド処理
        self.rebuild_line_cache(text);
        Ok(())
    }
}
```

## 10. 制限事項

### MVPでの制約
- SIMD最適化は未実装
- インクリメンタル更新は未実装
- 並列処理対応は未実装
- 複合文字（結合文字）の詳細対応は基本レベル

### 既知の制限
- 非常に長い行（>100,000文字）での性能制限
- メモリ使用量の最適化余地
- プラットフォーム固有の文字幅差異

この仕様により、Altreエディタのナビゲーションシステムに高速で正確な位置計算機能を提供し、ユーザーに快適な編集体験を実現する。