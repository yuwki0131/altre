# 検索・置換データ構造設計

## 概要
本ドキュメントは、altreの検索・置換機能で用いるデータ構造とアルゴリズムを整理する。MVPではリテラル検索を中心に据えつつ、将来の正規表現対応や大規模テキスト最適化を見据えた階層化された戦略を定義する。

## 設計方針
1. **単純→高度の段階的実装**: MVPではTwo-Wayアルゴリズムをベースにし、高度なアルゴリズムは順次追加。
2. **抽象化による拡張性**: `StringMatcher`トレイトを中心に、リテラル検索、正規表現検索、差分検索を差し替え可能にする。
3. **Unicode安全**: UTF-8バイト列をギャップバッファ上で扱う前提で、文字境界計算を共通化する。
4. **キャッシュと差分更新**: インクリメンタル検索の応答性確保のため、マッチ結果の再利用と差分更新を重視する。

## 基本データ構造
```rust
/// 検索オプション
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub normalize_unicode: bool,
    pub word_boundary: bool,
    pub full_width_sensitive: bool,
}

impl SearchOptions {
    pub fn for_pattern(pattern: &str) -> Self {
        let has_upper = pattern.chars().any(|c| c.is_uppercase());
        Self {
            case_sensitive: has_upper,
            normalize_unicode: false, // MVPは未対応
            word_boundary: false,
            full_width_sensitive: true,
        }
    }
}

/// マッチ結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub range: std::ops::Range<usize>,
    pub line: usize,
    pub column: usize,
}
```

## マッチング戦略
| 戦略 | 適用条件 | 実装ステータス |
| --- | --- | --- |
| NaiveMatcher | パターン長 < 8、テキスト長 < 4KB | MVPで実装 |
| TwoWayMatcher | デフォルト戦略（標準ライブラリ相当） | MVPで実装 |
| BoyerMooreMatcher | パターン長 > 64、繰り返し検索時 | 将来実装 |
| RegexMatcher | 正規表現検索 | 将来実装 |

```rust
pub enum SearchStrategy {
    Naive,
    TwoWay,
    BoyerMoore,
}

pub struct SearchAlgorithmManager;

impl SearchAlgorithmManager {
    pub fn select(pattern: &str, text_len: usize) -> SearchStrategy {
        match (pattern.len(), text_len) {
            (0..=7, 0..=4096) => SearchStrategy::Naive,
            (0..=64, _) => SearchStrategy::TwoWay,
            _ => SearchStrategy::BoyerMoore,
        }
    }
}
```

## キャッシュ設計
```rust
/// 検索結果キャッシュ（LRU）
pub struct SearchResultCache {
    capacity: usize,
    entries: std::collections::VecDeque<CachedMatches>,
}

pub struct CachedMatches {
    pub pattern: String,
    pub direction: SearchDirection,
    pub matches: Vec<SearchMatch>,
    pub timestamp: std::time::Instant,
}
```
- CapacityはMVPで16件に固定。
- `entries`は再利用の際に更新され、失効はLRUで行う。
- キャッシュヒット時でもテキストの変更検知が必要なため、`TextFingerprint`（行数・バージョン番号）を保持する。

## 差分更新
```rust
pub struct IncrementalMatchUpdater {
    previous_pattern: String,
    previous_matches: Vec<SearchMatch>,
}

impl IncrementalMatchUpdater {
    pub fn on_add_char(
        &mut self,
        new_char: char,
        text: &TextSnapshot,
        options: &SearchOptions,
    ) -> Vec<SearchMatch> {
        if self.previous_pattern.is_empty() {
            return full_scan(text, new_char.to_string(), options);
        }
        filter_existing(&self.previous_matches, new_char, text, options)
    }

    pub fn on_delete_char(
        &mut self,
        new_pattern: &str,
        text: &TextSnapshot,
        options: &SearchOptions,
    ) -> Vec<SearchMatch> {
        full_scan(text, new_pattern.to_string(), options)
    }
}
```
- `TextSnapshot`はギャップバッファから取得する読み取り専用ビュー。
- 追加時は既存マッチをフィルタリング、削除時は安全性を優先して再スキャン。

## 大規模テキスト対応（将来）
1. **ストリーミング検索**: `StreamingSearch<R: BufRead>`でチャンク再読み込みしながら検索。
2. **圧縮マッチ表現**: `CompactSearchMatch`と`CompressedSearchResults`でメモリ消費を削減。
3. **インデックス構築**: `SuffixArray`や`WaveletTree`の導入を将来検討。

## 置換向けデータ構造
```rust
pub struct ReplacePlan {
    pub matches: Vec<SearchMatch>,
    pub replacement: String,
}

pub struct ReplaceHistoryEntry {
    pub match_index: usize,
    pub original_text: String,
    pub replacement_text: String,
}
```
- クエリ置換では全マッチを事前計算し、ユーザー操作ごとに`ReplacePlan`内のインデックスを進める。
- `ReplaceHistoryEntry`は`C-g`キャンセル時に巻き戻すために使用する。

## ベンチマークと計測
- `SearchBenchmark`構造体でテキストサイズ／パターン長／期待ヒット数を管理。
- Criterionベースの測定で、`TwoWayMatcher`の1MBテキストにおける平均レイテンシを100ms未満とする。
- プロファイル結果は`performance_report.md`に追記する運用。

## 依存クレート
- `memchr`: バイト列検索の高速化（Naive/TwoWay補助）
- `unicode-segmentation`: 将来の単語境界検索に利用
- `regex`: 正規表現対応時に利用
- `unicode-normalization`: 正規化オプション実装時に利用

## 将来課題
- Unicode正規化（NFC/NFD）を用いた曖昧検索
- 全角／半角同一視の実装
- マルチバッファ検索のためのインデックス共有

