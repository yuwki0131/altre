# 検索・置換データ構造とアルゴリズム設計

## タスク概要
検索・置換機能で使用する効率的なデータ構造とアルゴリズムを設計・実装する。

## 目的
- 高速な文字列検索アルゴリズムの選定と実装
- メモリ効率的な検索結果管理
- 大きなファイルでも快適に動作する検索システム

## 文字列検索アルゴリズム設計

### 基本検索アルゴリズムの比較
1. **単純文字列検索（Naive Search）**
   - 短いパターン・小さなテキストに適用
   - O(n*m)の計算量

2. **Boyer-Moore法**
   - 長いパターンに効果的
   - 不良文字ヒューリスティック
   - 良好サフィックスヒューリスティック

3. **KMP法（Knuth-Morris-Pratt）**
   - 前処理でパターンの部分マッチテーブル構築
   - O(n+m)の線形時間計算量

4. **Two-Way アルゴリズム**
   - Rust標準ライブラリが採用
   - 空間効率と時間効率のバランス

### 実装するアルゴリズム選定
```rust
/// 文字列検索戦略の選択
pub enum SearchStrategy {
    /// 短いパターン用（< 8文字）
    Naive,
    /// 中程度のパターン用（8-64文字）
    TwoWay,
    /// 長いパターン用（> 64文字）
    BoyerMoore,
}

/// 検索アルゴリズムマネージャー
pub struct SearchAlgorithmManager;

impl SearchAlgorithmManager {
    /// パターンに最適なアルゴリズムを選択
    pub fn select_algorithm(pattern: &str) -> SearchStrategy {
        match pattern.len() {
            0..=7 => SearchStrategy::Naive,
            8..=64 => SearchStrategy::TwoWay,
            _ => SearchStrategy::BoyerMoore,
        }
    }
}
```

### Unicode対応設計
```rust
/// Unicode対応検索設定
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// 大文字小文字を区別するか
    pub case_sensitive: bool,

    /// Unicode正規化を行うか
    pub unicode_normalize: bool,

    /// 単語境界で検索するか
    pub word_boundary: bool,

    /// 全角半角を区別するか（日本語対応）
    pub full_half_width_sensitive: bool,
}

/// Unicode安全な文字列操作
pub struct UnicodeStringMatcher {
    options: SearchOptions,
}

impl UnicodeStringMatcher {
    /// Unicode正規化を適用した検索
    pub fn find_matches(&self, text: &str, pattern: &str) -> Vec<SearchMatch> {
        // NFD正規化適用
        let normalized_text = if self.options.unicode_normalize {
            unicode_normalization::UnicodeNormalization::nfd(text).collect::<String>()
        } else {
            text.to_string()
        };

        // 大文字小文字変換
        let (search_text, search_pattern) = if self.options.case_sensitive {
            (normalized_text, pattern.to_string())
        } else {
            (normalized_text.to_lowercase(), pattern.to_lowercase())
        };

        self.find_matches_impl(&search_text, &search_pattern)
    }
}
```

## 検索結果管理データ構造

### 効率的な検索結果キャッシュ
```rust
/// 検索結果のキャッシュ管理
pub struct SearchResultCache {
    /// パターンごとの検索結果キャッシュ
    cache: HashMap<String, CachedSearchResult>,

    /// LRUによるキャッシュサイズ制限
    lru: LinkedList<String>,

    /// 最大キャッシュサイズ
    max_cache_size: usize,
}

/// キャッシュされた検索結果
#[derive(Debug, Clone)]
pub struct CachedSearchResult {
    /// 検索結果
    pub matches: Vec<SearchMatch>,

    /// テキストのハッシュ値（変更検出用）
    pub text_hash: u64,

    /// 最終アクセス時刻
    pub last_accessed: std::time::Instant,
}

impl SearchResultCache {
    /// 検索結果をキャッシュから取得
    pub fn get(&mut self, pattern: &str, text_hash: u64) -> Option<&Vec<SearchMatch>> {
        let result = self.cache.get(pattern)?;

        // テキストが変更されていないかチェック
        if result.text_hash != text_hash {
            self.cache.remove(pattern);
            return None;
        }

        // LRU更新
        self.update_lru(pattern);
        Some(&result.matches)
    }

    /// 検索結果をキャッシュに保存
    pub fn insert(&mut self, pattern: String, matches: Vec<SearchMatch>, text_hash: u64) {
        // キャッシュサイズ制限チェック
        self.ensure_cache_size();

        let cached_result = CachedSearchResult {
            matches,
            text_hash,
            last_accessed: std::time::Instant::now(),
        };

        self.cache.insert(pattern.clone(), cached_result);
        self.lru.push_front(pattern);
    }
}
```

### インクリメンタル検索用差分更新
```rust
/// インクリメンタル検索の差分更新マネージャー
pub struct IncrementalSearchManager {
    /// 前回の検索状態
    previous_state: Option<IncrementalSearchState>,

    /// 検索結果の差分キャッシュ
    differential_cache: Vec<SearchMatch>,
}

impl IncrementalSearchManager {
    /// パターン追加時の効率的な検索結果更新
    pub fn update_for_pattern_addition(
        &mut self,
        new_char: char,
        current_matches: &[SearchMatch],
        text: &str,
    ) -> Vec<SearchMatch> {
        // 前回の検索結果から絞り込み
        if let Some(ref prev_state) = self.previous_state {
            // 新しい文字で既存マッチを絞り込み
            self.filter_matches_for_new_char(new_char, current_matches, text)
        } else {
            // 初回検索は全文検索
            self.full_search(text, &format!("{}", new_char))
        }
    }

    /// パターン削除時の検索結果復元
    pub fn update_for_pattern_deletion(
        &mut self,
        deleted_char: char,
        current_pattern: &str,
        text: &str,
    ) -> Vec<SearchMatch> {
        // より短いパターンでの再検索が必要
        self.full_search(text, current_pattern)
    }
}
```

## 大容量テキスト対応

### ストリーミング検索
```rust
/// 大容量ファイル用ストリーミング検索
pub struct StreamingSearch<R: std::io::BufRead> {
    reader: R,
    pattern: String,
    buffer: String,
    buffer_size: usize,
    overlap_size: usize,
}

impl<R: std::io::BufRead> StreamingSearch<R> {
    /// ストリーミング検索実行
    pub fn search(&mut self) -> Result<Vec<SearchMatch>, SearchError> {
        let mut all_matches = Vec::new();
        let mut global_offset = 0;

        loop {
            // バッファにデータ読み込み
            let bytes_read = self.read_chunk()?;
            if bytes_read == 0 {
                break;
            }

            // 現在のチャンクで検索実行
            let chunk_matches = self.search_chunk(&self.buffer, global_offset);
            all_matches.extend(chunk_matches);

            // オーバーラップ部分を保持してバッファを更新
            self.update_buffer_for_next_chunk();
            global_offset += bytes_read - self.overlap_size;
        }

        Ok(all_matches)
    }
}
```

### メモリ効率的な検索結果表現
```rust
/// 大容量ファイル用の検索結果圧縮表現
#[derive(Debug, Clone)]
pub struct CompactSearchMatch {
    /// マッチ位置（ファイル全体でのバイトオフセット）
    pub offset: u64,

    /// マッチ長（圧縮表現）
    pub length: u16,

    /// 行番号（差分エンコーディング）
    pub line_delta: u32,

    /// 列番号
    pub column: u16,
}

/// 検索結果のRLE（Run-Length Encoding）圧縮
pub struct CompressedSearchResults {
    /// 圧縮されたマッチデータ
    compressed_matches: Vec<u8>,

    /// インデックス（高速な位置検索用）
    index: Vec<(u64, usize)>, // (ファイル位置, 圧縮データ内位置)
}
```

## 正規表現エンジン統合

### regex crateとの統合
```rust
/// 正規表現検索ラッパー
pub struct RegexSearchEngine {
    /// コンパイル済み正規表現のキャッシュ
    regex_cache: HashMap<String, regex::Regex>,

    /// キャッシュサイズ制限
    max_cache_size: usize,
}

impl RegexSearchEngine {
    /// 正規表現パターンで検索
    pub fn find_matches(&mut self, text: &str, pattern: &str) -> Result<Vec<SearchMatch>, SearchError> {
        let regex = self.get_or_compile_regex(pattern)?;

        let matches: Vec<SearchMatch> = regex
            .find_iter(text)
            .map(|m| {
                let start = m.start();
                let end = m.end();
                let (line, column) = self.offset_to_line_column(text, start);

                SearchMatch {
                    start,
                    end,
                    line,
                    column,
                }
            })
            .collect();

        Ok(matches)
    }

    /// 正規表現の事前コンパイルとキャッシュ
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&regex::Regex, SearchError> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| SearchError::RegexError {
                    message: e.to_string()
                })?;

            // キャッシュサイズ制限
            if self.regex_cache.len() >= self.max_cache_size {
                self.evict_oldest_regex();
            }

            self.regex_cache.insert(pattern.to_string(), regex);
        }

        Ok(self.regex_cache.get(pattern).unwrap())
    }
}
```

## パフォーマンス測定・最適化

### ベンチマーク用データ構造
```rust
/// 検索パフォーマンス測定
pub struct SearchBenchmark {
    /// テストデータセット
    test_data: Vec<BenchmarkData>,
}

#[derive(Debug)]
pub struct BenchmarkData {
    /// テスト名
    pub name: String,

    /// テストテキスト
    pub text: String,

    /// 検索パターン
    pub pattern: String,

    /// 期待される結果数
    pub expected_matches: usize,
}

impl SearchBenchmark {
    /// パフォーマンステスト実行
    pub fn run_benchmarks(&self, search_engine: &mut dyn StringMatcher) -> BenchmarkResults {
        let mut results = Vec::new();

        for test_data in &self.test_data {
            let start_time = std::time::Instant::now();
            let matches = search_engine.find_matches(
                &test_data.text,
                &test_data.pattern,
                true
            );
            let elapsed = start_time.elapsed();

            results.push(BenchmarkResult {
                test_name: test_data.name.clone(),
                elapsed_time: elapsed,
                matches_found: matches.len(),
                expected_matches: test_data.expected_matches,
                memory_used: self.measure_memory_usage(),
            });
        }

        BenchmarkResults { results }
    }
}
```

## 依存関係
- unicode-normalization crate（Unicode正規化）
- regex crate（正規表現エンジン）
- memchr crate（高速バイト検索）

## 成果物
- 効率的な検索アルゴリズム実装
- 検索結果管理システム
- パフォーマンスベンチマーク

## 完了条件
- [ ] 基本文字列検索アルゴリズム実装完了
- [ ] Unicode対応検索機能実装完了
- [ ] 検索結果キャッシュシステム実装完了
- [ ] ベンチマークテスト実装完了
- [ ] 大容量ファイル対応機能実装完了

## 進捗記録
- 作成日：2025-01-28
- 状態：設計フェーズ