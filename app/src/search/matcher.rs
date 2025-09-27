//! 検索用マッチャー

use super::types::SearchMatch;

/// 文字列マッチング戦略
pub trait StringMatcher {
    /// 文字列内のすべてのマッチを返す
    fn find_matches(&self, text: &str, pattern: &str, case_sensitive: bool) -> Vec<SearchMatch>;
}

/// 単純なリテラルマッチャー（Two-Way相当の振る舞い）
#[derive(Debug, Default, Clone)]
pub struct LiteralMatcher;

impl LiteralMatcher {
    /// インスタンスを作成
    pub fn new() -> Self {
        Self
    }
}

impl StringMatcher for LiteralMatcher {
    fn find_matches(&self, text: &str, pattern: &str, case_sensitive: bool) -> Vec<SearchMatch> {
        if pattern.is_empty() {
            return Vec::new();
        }

        let chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        if pattern_chars.is_empty() || pattern_chars.len() > chars.len() {
            return Vec::new();
        }

        // 文字ごとの位置情報を前計算
        let mut line = 0usize;
        let mut column = 0usize;
        let mut line_map = Vec::with_capacity(chars.len());

        for ch in &chars {
            line_map.push((line, column));
            if *ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        let last_start = chars.len() - pattern_chars.len();
        let mut matches = Vec::new();

        'outer: for start in 0..=last_start {
            for (offset, pat_ch) in pattern_chars.iter().enumerate() {
                if !chars_equal(chars[start + offset], *pat_ch, case_sensitive) {
                    continue 'outer;
                }
            }

            let (line, column) = line_map[start];
            let end = start + pattern_chars.len();

            matches.push(SearchMatch {
                start,
                end,
                line,
                column,
            });
        }

        matches
    }
}

fn chars_equal(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        return a == b;
    }

    // Unicodeケースフォールディング（簡易）
    let lower_a = a.to_lowercase();
    let lower_b = b.to_lowercase();
    lower_a.to_string() == lower_b.to_string()
}

#[cfg(test)]
mod tests {
    use super::{LiteralMatcher, StringMatcher};

    #[test]
    fn finds_basic_matches() {
        let matcher = LiteralMatcher::new();
        let result = matcher.find_matches("hello world hello", "hello", true);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].start, 0);
        assert_eq!(result[1].start, 12);
    }

    #[test]
    fn handles_newlines() {
        let matcher = LiteralMatcher::new();
        let result = matcher.find_matches("hello\nworld", "world", true);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line, 1);
        assert_eq!(result[0].column, 0);
    }

    #[test]
    fn returns_empty_for_non_match() {
        let matcher = LiteralMatcher::new();
        let result = matcher.find_matches("abc", "z", true);
        assert!(result.is_empty());
    }

    #[test]
    fn supports_case_insensitive() {
        let matcher = LiteralMatcher::new();
        let result = matcher.find_matches("Hello World", "hello", false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].start, 0);
    }
}
