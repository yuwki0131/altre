# 正規表現検索・置換機能実装

## タスク概要
正規表現による検索・置換機能（C-M-s、C-M-r、C-M-%）の実装を行う。

## 目的
- 正規表現パターンによる高度な検索・置換機能の提供
- キャプチャグループを使用した置換機能の実装
- 安全で効率的な正規表現処理の実現

## 実装対象機能

### 正規表現検索機能
1. **正規表現インクリメンタル検索**
   - `C-M-s`：前方正規表現検索
   - `C-M-r`：後方正規表現検索

2. **正規表現パターン**
   - 基本的なメタ文字（`.`, `*`, `+`, `?`, `^`, `$`）
   - 文字クラス（`[...]`, `[^...]`）
   - キャプチャグループ（`(...)`, `(?:...)`）
   - エスケープシーケンス（`\d`, `\w`, `\s`等）

### 正規表現置換機能
1. **正規表現置換（C-M-%）**
   - パターンマッチング置換
   - キャプチャグループ参照（`$1`, `$2`, `\1`, `\2`）

2. **高度な置換パターン**
   - 条件付き置換
   - 大文字小文字変換（`\u`, `\l`, `\U`, `\L`）

## モジュール構造

```
search/
├── regex.rs                # 正規表現エンジン統合
├── regex_matcher.rs        # 正規表現マッチャー
├── regex_replace.rs        # 正規表現置換
└── regex_patterns.rs       # 正規表現パターン管理
```

## 実装詳細

### 正規表現エンジン統合
```rust
// app/src/search/regex.rs

use regex::{Regex, RegexBuilder, Captures};
use crate::error::{Result, AltreError, SearchError};
use super::{SearchMatch, StringMatcher, SearchDirection};

/// 正規表現検索エンジン
pub struct RegexSearchEngine {
    /// コンパイル済み正規表現のキャッシュ
    regex_cache: HashMap<String, CachedRegex>,

    /// 検索オプション
    options: RegexOptions,

    /// キャッシュサイズ制限
    max_cache_size: usize,
}

/// キャッシュされた正規表現
#[derive(Debug, Clone)]
struct CachedRegex {
    /// コンパイル済み正規表現
    regex: Regex,

    /// 最終使用時刻
    last_used: std::time::Instant,

    /// 使用回数
    use_count: u32,
}

/// 正規表現オプション
#[derive(Debug, Clone)]
pub struct RegexOptions {
    /// 大文字小文字を区別しない
    pub case_insensitive: bool,

    /// 複数行モード（^$が行頭行末にマッチ）
    pub multi_line: bool,

    /// ドット改行モード（.が改行にマッチ）
    pub dot_matches_new_line: bool,

    /// Unicode対応
    pub unicode: bool,

    /// 空文字列マッチを許可
    pub allow_empty_matches: bool,
}

impl Default for RegexOptions {
    fn default() -> Self {
        Self {
            case_insensitive: false,
            multi_line: true,
            dot_matches_new_line: false,
            unicode: true,
            allow_empty_matches: false,
        }
    }
}

impl RegexSearchEngine {
    pub fn new() -> Self {
        Self {
            regex_cache: HashMap::new(),
            options: RegexOptions::default(),
            max_cache_size: 50, // 最大50個の正規表現をキャッシュ
        }
    }

    /// 正規表現パターンをコンパイル
    pub fn compile_pattern(&mut self, pattern: &str) -> Result<&Regex> {
        // キャッシュから検索
        if let Some(cached) = self.regex_cache.get_mut(pattern) {
            cached.last_used = std::time::Instant::now();
            cached.use_count += 1;
            return Ok(&cached.regex);
        }

        // 新規コンパイル
        let regex = self.build_regex(pattern)?;

        // キャッシュサイズ制限チェック
        if self.regex_cache.len() >= self.max_cache_size {
            self.evict_least_used();
        }

        let cached_regex = CachedRegex {
            regex,
            last_used: std::time::Instant::now(),
            use_count: 1,
        };

        self.regex_cache.insert(pattern.to_string(), cached_regex);
        Ok(&self.regex_cache[pattern].regex)
    }

    /// 正規表現ビルダーでパターンをコンパイル
    fn build_regex(&self, pattern: &str) -> Result<Regex> {
        let mut builder = RegexBuilder::new(pattern);

        builder
            .case_insensitive(self.options.case_insensitive)
            .multi_line(self.options.multi_line)
            .dot_matches_new_line(self.options.dot_matches_new_line)
            .unicode(self.options.unicode);

        // 空文字列マッチの制御
        if !self.options.allow_empty_matches {
            // 空文字列マッチを検出するためのポストプロセシング
        }

        builder.build()
            .map_err(|e| AltreError::Search(SearchError::RegexError {
                message: format!("正規表現コンパイルエラー: {}", e)
            }))
    }

    /// 最も使用頻度の低い正規表現をキャッシュから削除
    fn evict_least_used(&mut self) {
        if let Some((pattern, _)) = self.regex_cache
            .iter()
            .min_by_key(|(_, cached)| (cached.use_count, cached.last_used))
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            self.regex_cache.remove(&pattern);
        }
    }
}

impl StringMatcher for RegexSearchEngine {
    fn find_matches(&mut self, text: &str, pattern: &str, _case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let regex = self.compile_pattern(pattern)?;
        let matches: Vec<SearchMatch> = regex
            .find_iter(text)
            .enumerate()
            .filter_map(|(index, m)| {
                let start = m.start();
                let end = m.end();

                // 空文字列マッチのフィルタリング
                if !self.options.allow_empty_matches && start == end {
                    return None;
                }

                let (line, column) = self.offset_to_line_column(text, start);
                Some(SearchMatch {
                    start,
                    end,
                    line,
                    column,
                    match_text: m.as_str().to_string(),
                    captures: self.extract_captures(regex, text, start),
                })
            })
            .collect();

        Ok(matches)
    }

    fn find_next(
        &mut self,
        text: &str,
        pattern: &str,
        start: usize,
        direction: SearchDirection,
        _case_sensitive: bool,
    ) -> Result<Option<SearchMatch>> {
        let regex = self.compile_pattern(pattern)?;

        match direction {
            SearchDirection::Forward => {
                if let Some(m) = regex.find_at(text, start) {
                    let (line, column) = self.offset_to_line_column(text, m.start());
                    Ok(Some(SearchMatch {
                        start: m.start(),
                        end: m.end(),
                        line,
                        column,
                        match_text: m.as_str().to_string(),
                        captures: self.extract_captures(regex, text, m.start()),
                    }))
                } else {
                    Ok(None)
                }
            },
            SearchDirection::Backward => {
                // 後方検索は start まで全て検索して最後のものを取る
                let matches: Vec<_> = regex.find_iter(&text[..start]).collect();
                if let Some(last_match) = matches.last() {
                    let (line, column) = self.offset_to_line_column(text, last_match.start());
                    Ok(Some(SearchMatch {
                        start: last_match.start(),
                        end: last_match.end(),
                        line,
                        column,
                        match_text: last_match.as_str().to_string(),
                        captures: self.extract_captures(regex, text, last_match.start()),
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

impl RegexSearchEngine {
    /// キャプチャグループを抽出
    fn extract_captures(&self, regex: &Regex, text: &str, match_start: usize) -> Option<Vec<String>> {
        if let Some(captures) = regex.captures_at(text, match_start) {
            Some(captures
                .iter()
                .skip(1) // 最初はフルマッチなのでスキップ
                .map(|cap| cap.map_or(String::new(), |m| m.as_str().to_string()))
                .collect())
        } else {
            None
        }
    }

    /// オフセットから行・列番号を計算
    fn offset_to_line_column(&self, text: &str, offset: usize) -> (usize, usize) {
        let mut line = 0;
        let mut column = 0;

        for (i, ch) in text.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        (line, column)
    }
}

/// 拡張された検索マッチ（キャプチャ情報付き）
#[derive(Debug, Clone)]
pub struct RegexSearchMatch {
    /// 基本マッチ情報
    pub base: SearchMatch,

    /// マッチしたテキスト
    pub match_text: String,

    /// キャプチャグループ
    pub captures: Vec<String>,
}
```

### 正規表現置換実装
```rust
// app/src/search/regex_replace.rs

use regex::{Regex, Captures};
use crate::error::Result;
use super::{RegexSearchEngine, ReplaceEngine};

/// 正規表現置換エンジン
pub struct RegexReplaceEngine {
    /// 検索エンジン
    search_engine: RegexSearchEngine,

    /// 基本置換エンジン
    base_engine: ReplaceEngine,
}

impl RegexReplaceEngine {
    pub fn new() -> Self {
        Self {
            search_engine: RegexSearchEngine::new(),
            base_engine: ReplaceEngine::new(),
        }
    }

    /// 正規表現置換開始
    pub fn start_regex_replace(
        &mut self,
        editor: &crate::buffer::TextEditor,
        pattern: String,
        replacement: String,
    ) -> Result<RegexReplaceResult> {
        // 正規表現パターンをコンパイル
        let regex = self.search_engine.compile_pattern(&pattern)?;

        // 全マッチを検索（キャプチャ情報付き）
        let text = editor.text();
        let matches = self.find_all_matches_with_captures(text, regex)?;

        if matches.is_empty() {
            return Ok(RegexReplaceResult::NotFound);
        }

        // 置換文字列のテンプレートを解析
        let replacement_template = ReplacementTemplate::parse(&replacement)?;

        // 基本置換エンジンを初期化
        self.base_engine.start_replace(
            editor,
            pattern.clone(),
            String::new(), // テンプレート適用後に設定
        )?;

        Ok(RegexReplaceResult::Started {
            pattern,
            replacement_template,
            matches,
            total_matches: matches.len(),
        })
    }

    /// キャプチャ付きマッチを全て検索
    fn find_all_matches_with_captures(
        &mut self,
        text: &str,
        regex: &Regex,
    ) -> Result<Vec<RegexMatch>> {
        let matches: Vec<RegexMatch> = regex
            .captures_iter(text)
            .map(|caps| {
                let full_match = caps.get(0).unwrap();
                let captures: Vec<String> = caps
                    .iter()
                    .skip(1)
                    .map(|cap| cap.map_or(String::new(), |m| m.as_str().to_string()))
                    .collect();

                RegexMatch {
                    start: full_match.start(),
                    end: full_match.end(),
                    text: full_match.as_str().to_string(),
                    captures,
                }
            })
            .collect();

        Ok(matches)
    }

    /// 現在のマッチを置換（キャプチャ適用）
    pub fn replace_current_with_captures(
        &mut self,
        editor: &mut crate::buffer::TextEditor,
        replacement_template: &ReplacementTemplate,
        current_match: &RegexMatch,
    ) -> Result<String> {
        // テンプレートにキャプチャを適用
        let actual_replacement = replacement_template.apply(&current_match.captures)?;

        // 基本置換エンジンで置換実行
        editor.replace_range(
            current_match.start,
            current_match.end,
            &actual_replacement,
        )?;

        Ok(actual_replacement)
    }
}

/// 正規表現マッチ情報
#[derive(Debug, Clone)]
pub struct RegexMatch {
    /// 開始位置
    pub start: usize,

    /// 終了位置
    pub end: usize,

    /// マッチしたテキスト
    pub text: String,

    /// キャプチャグループ
    pub captures: Vec<String>,
}

/// 置換テンプレート
#[derive(Debug, Clone)]
pub struct ReplacementTemplate {
    /// テンプレート要素
    elements: Vec<TemplateElement>,
}

/// テンプレート要素
#[derive(Debug, Clone)]
enum TemplateElement {
    /// リテラル文字列
    Literal(String),

    /// キャプチャ参照（$1, $2, \1, \2）
    Capture(usize),

    /// 大文字変換（\u）
    UppercaseNext,

    /// 小文字変換（\l）
    LowercaseNext,

    /// 全て大文字変換（\U）
    UppercaseAll,

    /// 全て小文字変換（\L）
    LowercaseAll,

    /// 変換終了（\E）
    EndConversion,
}

impl ReplacementTemplate {
    /// 置換テンプレートをパース
    pub fn parse(template: &str) -> Result<Self> {
        let mut elements = Vec::new();
        let mut chars = template.chars().peekable();
        let mut current_literal = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    if let Some(&next_ch) = chars.peek() {
                        match next_ch {
                            '1'..='9' => {
                                // \1, \2, ... キャプチャ参照
                                if !current_literal.is_empty() {
                                    elements.push(TemplateElement::Literal(current_literal.clone()));
                                    current_literal.clear();
                                }
                                chars.next(); // 数字を消費
                                let capture_num = next_ch.to_digit(10).unwrap() as usize;
                                elements.push(TemplateElement::Capture(capture_num));
                            },
                            'u' => {
                                // \u 次の文字を大文字に
                                if !current_literal.is_empty() {
                                    elements.push(TemplateElement::Literal(current_literal.clone()));
                                    current_literal.clear();
                                }
                                chars.next();
                                elements.push(TemplateElement::UppercaseNext);
                            },
                            'l' => {
                                // \l 次の文字を小文字に
                                if !current_literal.is_empty() {
                                    elements.push(TemplateElement::Literal(current_literal.clone()));
                                    current_literal.clear();
                                }
                                chars.next();
                                elements.push(TemplateElement::LowercaseNext);
                            },
                            'U' => {
                                // \U 以降全て大文字に
                                if !current_literal.is_empty() {
                                    elements.push(TemplateElement::Literal(current_literal.clone()));
                                    current_literal.clear();
                                }
                                chars.next();
                                elements.push(TemplateElement::UppercaseAll);
                            },
                            'L' => {
                                // \L 以降全て小文字に
                                if !current_literal.is_empty() {
                                    elements.push(TemplateElement::Literal(current_literal.clone()));
                                    current_literal.clear();
                                }
                                chars.next();
                                elements.push(TemplateElement::LowercaseAll);
                            },
                            'E' => {
                                // \E 大文字小文字変換終了
                                if !current_literal.is_empty() {
                                    elements.push(TemplateElement::Literal(current_literal.clone()));
                                    current_literal.clear();
                                }
                                chars.next();
                                elements.push(TemplateElement::EndConversion);
                            },
                            _ => {
                                // 通常のエスケープ
                                chars.next();
                                current_literal.push(next_ch);
                            }
                        }
                    } else {
                        current_literal.push(ch);
                    }
                },
                '$' => {
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() {
                            // $1, $2, ... キャプチャ参照
                            if !current_literal.is_empty() {
                                elements.push(TemplateElement::Literal(current_literal.clone()));
                                current_literal.clear();
                            }
                            chars.next();
                            let capture_num = next_ch.to_digit(10).unwrap() as usize;
                            elements.push(TemplateElement::Capture(capture_num));
                        } else {
                            current_literal.push(ch);
                        }
                    } else {
                        current_literal.push(ch);
                    }
                },
                _ => current_literal.push(ch),
            }
        }

        if !current_literal.is_empty() {
            elements.push(TemplateElement::Literal(current_literal));
        }

        Ok(ReplacementTemplate { elements })
    }

    /// キャプチャを適用して実際の置換文字列を生成
    pub fn apply(&self, captures: &[String]) -> Result<String> {
        let mut result = String::new();
        let mut case_conversion = CaseConversion::None;

        for element in &self.elements {
            match element {
                TemplateElement::Literal(text) => {
                    result.push_str(&self.apply_case_conversion(text, &mut case_conversion));
                },
                TemplateElement::Capture(n) => {
                    if *n > 0 && *n <= captures.len() {
                        let capture_text = &captures[*n - 1];
                        result.push_str(&self.apply_case_conversion(capture_text, &mut case_conversion));
                    }
                    // キャプチャが存在しない場合は何もしない
                },
                TemplateElement::UppercaseNext => {
                    case_conversion = CaseConversion::UppercaseNext;
                },
                TemplateElement::LowercaseNext => {
                    case_conversion = CaseConversion::LowercaseNext;
                },
                TemplateElement::UppercaseAll => {
                    case_conversion = CaseConversion::UppercaseAll;
                },
                TemplateElement::LowercaseAll => {
                    case_conversion = CaseConversion::LowercaseAll;
                },
                TemplateElement::EndConversion => {
                    case_conversion = CaseConversion::None;
                },
            }
        }

        Ok(result)
    }

    /// 大文字小文字変換を適用
    fn apply_case_conversion(&self, text: &str, conversion: &mut CaseConversion) -> String {
        match conversion {
            CaseConversion::None => text.to_string(),
            CaseConversion::UppercaseNext => {
                *conversion = CaseConversion::None;
                if let Some(first_char) = text.chars().next() {
                    first_char.to_uppercase().chain(text.chars().skip(1)).collect()
                } else {
                    text.to_string()
                }
            },
            CaseConversion::LowercaseNext => {
                *conversion = CaseConversion::None;
                if let Some(first_char) = text.chars().next() {
                    first_char.to_lowercase().chain(text.chars().skip(1)).collect()
                } else {
                    text.to_string()
                }
            },
            CaseConversion::UppercaseAll => text.to_uppercase(),
            CaseConversion::LowercaseAll => text.to_lowercase(),
        }
    }
}

/// 大文字小文字変換状態
#[derive(Debug, Clone)]
enum CaseConversion {
    None,
    UppercaseNext,
    LowercaseNext,
    UppercaseAll,
    LowercaseAll,
}

/// 正規表現置換結果
#[derive(Debug, Clone)]
pub enum RegexReplaceResult {
    /// 置換開始
    Started {
        pattern: String,
        replacement_template: ReplacementTemplate,
        matches: Vec<RegexMatch>,
        total_matches: usize,
    },

    /// マッチなし
    NotFound,

    /// 置換完了
    Completed {
        replaced_count: usize,
        total_matches: usize,
    },
}
```

## テスト実装

### 正規表現検索テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_basic_search() {
        let mut engine = RegexSearchEngine::new();

        let matches = engine.find_matches(
            "hello123 world456",
            r"\d+",
            true
        ).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 5);
        assert_eq!(matches[0].end, 8);
        assert_eq!(matches[0].match_text, "123");
    }

    #[test]
    fn test_regex_capture_groups() {
        let mut engine = RegexSearchEngine::new();

        let matches = engine.find_matches(
            "name: John, age: 25",
            r"(\w+): (\w+)",
            true
        ).unwrap();

        assert_eq!(matches.len(), 2);
        let first_match = &matches[0];
        assert_eq!(first_match.captures.as_ref().unwrap().len(), 2);
        assert_eq!(first_match.captures.as_ref().unwrap()[0], "name");
        assert_eq!(first_match.captures.as_ref().unwrap()[1], "John");
    }

    #[test]
    fn test_replacement_template() {
        let template = ReplacementTemplate::parse(r"$2, $1").unwrap();
        let captures = vec!["John".to_string(), "Doe".to_string()];
        let result = template.apply(&captures).unwrap();
        assert_eq!(result, "Doe, John");
    }

    #[test]
    fn test_case_conversion() {
        let template = ReplacementTemplate::parse(r"\u$1\L$2").unwrap();
        let captures = vec!["hello".to_string(), "WORLD".to_string()];
        let result = template.apply(&captures).unwrap();
        assert_eq!(result, "Helloworld");
    }
}
```

## 依存関係
- regex crate
- 基本検索・置換システム
- 文字列処理ユーティリティ

## 成果物
- 正規表現検索エンジン
- 正規表現置換エンジン
- キャプチャグループ処理
- 置換テンプレートシステム

## 完了条件
- [ ] RegexSearchEngine実装完了
- [ ] RegexReplaceEngine実装完了
- [ ] キャプチャグループ機能動作確認
- [ ] 置換テンプレート機能動作確認
- [ ] 大文字小文字変換機能動作確認
- [ ] 正規表現キャッシュ機能動作確認
- [ ] 単体・統合テスト実装完了
- [ ] エラーハンドリング実装完了

## 進捗記録
- 作成日：2025-01-28
- 状態：実装準備完了

## ステータス
- `regex` クレートは `app/Cargo.toml:1` にまだ依存追加されていないため、正規表現 API の下地が未整備。
- 現行の検索機能は `app/src/search/` のプレーンマッチのみで、キャプチャやテンプレート置換の入り口が存在しない。
- エラー設計 (`docs/design/error_handling.md:21`) に検索系エラー区分がなく、regex 例外の取り扱いが未反映。導入前に補完が必要。

## 次アクション
1. `regex` クレートと `regex-syntax` の依存を追加し、`app/src/search/regex.rs` などモジュールを作成。
2. 既存検索インターフェース (`app/src/search/mod.rs`) に正規表現モードを統合し、UI 操作 (`C-M-s` など) を実装。
3. `app/tests/` と `app/benches/` に正規表現モードのテスト・ベンチマークを追加し、パフォーマンス影響を検証。
