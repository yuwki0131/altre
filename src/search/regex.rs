use regex::{Captures, RegexBuilder};
use thiserror::Error;

pub type RegexError = regex::Error;

#[derive(Debug, Error)]
pub enum TemplateParseError {
    #[error("無効なグループ指定: {0}")]
    InvalidGroup(String),
}

#[derive(Debug, Clone)]
pub struct RegexCandidate {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone)]
struct ReplacementTemplate {
    parts: Vec<TemplatePart>,
}

#[derive(Debug, Clone)]
enum TemplatePart {
    Literal(String),
    Group(usize),
}

impl ReplacementTemplate {
    fn parse(template: &str) -> Result<Self, TemplateParseError> {
        let mut chars = template.chars().peekable();
        let mut parts = Vec::new();
        let mut literal = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '$' => {
                    if let Some(&next) = chars.peek() {
                        if next == '$' {
                            literal.push('$');
                            chars.next();
                        } else if next.is_ascii_digit() {
                            if !literal.is_empty() {
                                parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                            }
                            let mut digits = String::new();
                            while let Some(&digit) = chars.peek() {
                                if digit.is_ascii_digit() {
                                    digits.push(digit);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            if digits.is_empty() {
                                literal.push('$');
                                continue;
                            }
                            let index = digits
                                .parse::<usize>()
                                .map_err(|_| TemplateParseError::InvalidGroup(digits.clone()))?;
                            parts.push(TemplatePart::Group(index));
                        } else {
                            literal.push('$');
                        }
                    } else {
                        literal.push('$');
                    }
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        literal.push(next);
                    } else {
                        literal.push('\\');
                    }
                }
                other => literal.push(other),
            }
        }

        if !literal.is_empty() {
            parts.push(TemplatePart::Literal(literal));
        }

        Ok(Self { parts })
    }

    fn render(&self, captures: &Captures<'_>) -> String {
        let mut output = String::new();
        for part in &self.parts {
            match part {
                TemplatePart::Literal(text) => output.push_str(text),
                TemplatePart::Group(index) => {
                    if let Some(mat) = captures.get(*index) {
                        output.push_str(mat.as_str());
                    }
                }
            }
        }
        output
    }
}

pub fn build_regex_candidates(
    pattern: &str,
    replacement: &str,
    text: &str,
    case_sensitive: bool,
) -> Result<Vec<RegexCandidate>, RegexError> {
    let regex = RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .multi_line(true)
        .dot_matches_new_line(false)
        .build()?;

    let template = ReplacementTemplate::parse(replacement)
        .map_err(|err| RegexError::Syntax(err.to_string()))?;

    let mut results = Vec::new();

    for captures in regex.captures_iter(text) {
        if let Some(mat) = captures.get(0) {
            let start_byte = mat.start();
            let end_byte = mat.end();
            let start_char = text[..start_byte].chars().count();
            let end_char = start_char + text[start_byte..end_byte].chars().count();
            let replacement_text = template.render(&captures);
            results.push(RegexCandidate {
                start: start_char,
                end: end_char,
                replacement: replacement_text,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn template_parses_literals_and_groups() {
        let template = ReplacementTemplate::parse("prefix-$1-$2").unwrap();
        let regex = Regex::new("(a)(b)").unwrap();
        let caps = regex.captures("ab").unwrap();
        assert_eq!(template.render(&caps), "prefix-a-b");
    }

    #[test]
    fn build_candidates_collects_ranges() {
        let candidates = build_regex_candidates(
            "(\\d+)",
            "[$1]",
            "id=42 and 100",
            true,
        )
        .unwrap();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].replacement, "[42]");
    }
}
