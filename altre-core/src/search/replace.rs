use super::matcher::{LiteralMatcher, StringMatcher};
use super::regex::{build_regex_candidates, RegexError};
use super::types::{HighlightKind, SearchHighlight};
use crate::buffer::TextEditor;
use crate::error::Result;

/// クエリ置換の開始結果
#[derive(Debug, Clone)]
pub struct ReplaceStart {
    pub total_matches: usize,
    pub case_sensitive: bool,
    pub is_regex: bool,
}

/// 置換進行状況
#[derive(Debug, Clone)]
pub struct ReplaceProgress {
    pub replaced: usize,
    pub skipped: usize,
    pub remaining: usize,
    pub finished: bool,
}

/// 置換セッション完了時のサマリ
#[derive(Debug, Clone)]
pub struct ReplaceSummary {
    pub replaced: usize,
    pub skipped: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone)]
struct ReplaceCandidate {
    start: usize,
    end: usize,
    replacement: String,
}

#[derive(Debug, Clone)]
struct AppliedChange {
    start: usize,
    replacement_len: usize,
    original: String,
}

#[derive(Debug, Clone)]
struct ReplaceState {
    pattern: String,
    replacement_input: String,
    candidates: Vec<ReplaceCandidate>,
    current_index: usize,
    replaced: usize,
    skipped: usize,
    is_regex: bool,
    history: Vec<AppliedChange>,
    total_initial: usize,
}

/// クエリ置換コントローラー
#[derive(Debug, Default, Clone)]
pub struct QueryReplaceController {
    state: Option<ReplaceState>,
}

impl QueryReplaceController {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    pub fn pattern(&self) -> Option<&str> {
        self.state.as_ref().map(|s| s.pattern.as_str())
    }

    pub fn replacement(&self) -> Option<&str> {
        self.state.as_ref().map(|s| s.replacement_input.as_str())
    }

    pub fn is_regex(&self) -> bool {
        self.state.as_ref().map(|s| s.is_regex).unwrap_or(false)
    }

    pub fn total_matches(&self) -> usize {
        self.state.as_ref().map(|s| s.total_initial).unwrap_or(0)
    }

    pub fn current_range(&self) -> Option<(usize, usize)> {
        let state = self.state.as_ref()?;
        let candidate = state.candidates.get(state.current_index)?;
        Some((candidate.start, candidate.end))
    }

    pub fn start_literal(
        &mut self,
        text: &str,
        pattern: String,
        replacement: String,
        case_sensitive: bool,
    ) -> ReplaceStart {
        let matcher = LiteralMatcher::new();
        let matches = matcher.find_matches(text, &pattern, case_sensitive);
        let total = matches.len();
        let candidates = matches
            .into_iter()
            .map(|m| ReplaceCandidate {
                start: m.start,
                end: m.end,
                replacement: replacement.clone(),
            })
            .collect::<Vec<_>>();

        self.state = if total == 0 {
            None
        } else {
            Some(ReplaceState {
                pattern: pattern.clone(),
                replacement_input: replacement.clone(),
                candidates,
                current_index: 0,
                replaced: 0,
                skipped: 0,
                is_regex: false,
                history: Vec::new(),
                total_initial: total,
            })
        };

        ReplaceStart {
            total_matches: total,
            case_sensitive,
            is_regex: false,
        }
    }

    pub fn start_regex(
        &mut self,
        text: &str,
        pattern: String,
        replacement: String,
        case_sensitive: bool,
    ) -> std::result::Result<ReplaceStart, RegexError> {
        let regex_matches = build_regex_candidates(&pattern, &replacement, text, case_sensitive)?;
        let total = regex_matches.len();
        let candidates = regex_matches
            .into_iter()
            .map(|m| ReplaceCandidate {
                start: m.start,
                end: m.end,
                replacement: m.replacement,
            })
            .collect::<Vec<_>>();

        self.state = if total == 0 {
            None
        } else {
            Some(ReplaceState {
                pattern: pattern.clone(),
                replacement_input: replacement.clone(),
                candidates,
                current_index: 0,
                replaced: 0,
                skipped: 0,
                is_regex: true,
                history: Vec::new(),
                total_initial: total,
            })
        };

        Ok(ReplaceStart {
            total_matches: total,
            case_sensitive,
            is_regex: true,
        })
    }

    pub fn current_preview(&self, text: &str) -> Option<(String, String, usize, usize)> {
        let state = self.state.as_ref()?;
        let candidate = state.candidates.get(state.current_index)?;
        let original = slice_by_char_indices(text, candidate.start, candidate.end);
        let total = state.total_initial;
        let index = state.replaced + state.skipped + 1;
        Some((original, candidate.replacement.clone(), index, total))
    }

    pub fn highlights(&self, text: &str) -> Vec<SearchHighlight> {
        let mut highlights = Vec::new();
        let Some(state) = &self.state else {
            return highlights;
        };

        for (idx, candidate) in state.candidates.iter().enumerate() {
            let (line, column) = line_column_at(text, candidate.start);
            let span = highlight_span(text, candidate.start, candidate.end);
            if span == 0 {
                continue;
            }
            highlights.push(SearchHighlight {
                line,
                start_column: column,
                end_column: column + span,
                is_current: idx == state.current_index,
                kind: HighlightKind::Search,
            });
        }

        highlights
    }

    pub fn accept_current(&mut self, editor: &mut TextEditor) -> Result<ReplaceProgress> {
        let Some(state) = self.state.as_mut() else {
            return Ok(ReplaceProgress {
                replaced: 0,
                skipped: 0,
                remaining: 0,
                finished: true,
            });
        };

        if state.candidates.is_empty() || state.current_index >= state.candidates.len() {
            return Ok(ReplaceProgress {
                replaced: state.replaced,
                skipped: state.skipped,
                remaining: 0,
                finished: true,
            });
        }

        let candidate = state.candidates.remove(state.current_index);
        let replacement_len = candidate.replacement.chars().count();
        let original =
            editor.replace_range_span(candidate.start, candidate.end, &candidate.replacement)?;
        let original_len = original.chars().count();
        let diff = replacement_len as isize - original_len as isize;

        state.history.push(AppliedChange {
            start: candidate.start,
            replacement_len,
            original,
        });
        state.replaced += 1;

        if diff != 0 {
            adjust_candidates(&mut state.candidates, state.current_index, diff);
        }

        if state.current_index >= state.candidates.len() && !state.candidates.is_empty() {
            state.current_index = state.candidates.len() - 1;
        }

        Ok(ReplaceProgress {
            replaced: state.replaced,
            skipped: state.skipped,
            remaining: state.candidates.len().saturating_sub(state.current_index),
            finished: state.candidates.is_empty(),
        })
    }

    pub fn skip_current(&mut self) -> ReplaceProgress {
        let Some(state) = self.state.as_mut() else {
            return ReplaceProgress {
                replaced: 0,
                skipped: 0,
                remaining: 0,
                finished: true,
            };
        };

        if state.candidates.is_empty() || state.current_index >= state.candidates.len() {
            return ReplaceProgress {
                replaced: state.replaced,
                skipped: state.skipped,
                remaining: 0,
                finished: true,
            };
        }

        state.candidates.remove(state.current_index);
        state.skipped += 1;

        if state.current_index >= state.candidates.len() && !state.candidates.is_empty() {
            state.current_index = state.candidates.len() - 1;
        }

        ReplaceProgress {
            replaced: state.replaced,
            skipped: state.skipped,
            remaining: state.candidates.len().saturating_sub(state.current_index),
            finished: state.candidates.is_empty(),
        }
    }

    pub fn accept_all(&mut self, editor: &mut TextEditor) -> Result<ReplaceProgress> {
        while self.is_active() {
            let finished = {
                let state = self.state.as_ref().unwrap();
                state.candidates.is_empty()
            };
            if finished {
                break;
            }
            self.accept_current(editor)?;
        }
        Ok(self.progress())
    }

    pub fn progress(&self) -> ReplaceProgress {
        if let Some(state) = &self.state {
            ReplaceProgress {
                replaced: state.replaced,
                skipped: state.skipped,
                remaining: state.candidates.len().saturating_sub(state.current_index),
                finished: state.candidates.is_empty(),
            }
        } else {
            ReplaceProgress {
                replaced: 0,
                skipped: 0,
                remaining: 0,
                finished: true,
            }
        }
    }

    pub fn cancel(&mut self, editor: &mut TextEditor) -> Result<ReplaceSummary> {
        if let Some(state) = self.state.take() {
            for change in state.history.iter().rev() {
                let end = change.start + change.replacement_len;
                editor.replace_range_span(change.start, end, &change.original)?;
            }
            Ok(ReplaceSummary {
                replaced: state.replaced,
                skipped: state.skipped,
                cancelled: true,
            })
        } else {
            Ok(ReplaceSummary {
                replaced: 0,
                skipped: 0,
                cancelled: true,
            })
        }
    }

    pub fn finish(&mut self) -> ReplaceSummary {
        if let Some(state) = self.state.take() {
            ReplaceSummary {
                replaced: state.replaced,
                skipped: state.skipped,
                cancelled: false,
            }
        } else {
            ReplaceSummary {
                replaced: 0,
                skipped: 0,
                cancelled: false,
            }
        }
    }
}

fn adjust_candidates(candidates: &mut [ReplaceCandidate], start_index: usize, diff: isize) {
    if diff == 0 {
        return;
    }

    for candidate in &mut candidates[start_index..] {
        candidate.start = offset(candidate.start, diff);
        candidate.end = offset(candidate.end, diff);
    }
}

fn offset(value: usize, diff: isize) -> usize {
    if diff >= 0 {
        value.saturating_add(diff as usize)
    } else {
        value.saturating_sub(diff.unsigned_abs() as usize)
    }
}

fn slice_by_char_indices(text: &str, start: usize, end: usize) -> String {
    let start_byte = char_to_byte_index(text, start);
    let end_byte = char_to_byte_index(text, end);
    text[start_byte..end_byte].to_string()
}

fn char_to_byte_index(text: &str, index: usize) -> usize {
    if index == 0 {
        return 0;
    }
    let mut count = 0usize;
    for (byte_idx, _) in text.char_indices() {
        if count == index {
            return byte_idx;
        }
        count += 1;
    }
    text.len()
}

fn line_column_at(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut column = 0usize;
    for (idx, ch) in text.chars().enumerate() {
        if idx == char_index {
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

fn highlight_span(text: &str, start: usize, end: usize) -> usize {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .take_while(|ch| *ch != '\n')
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_start_and_accept() {
        let mut editor = TextEditor::from_str("hello world hello");
        let mut controller = QueryReplaceController::new();
        let start = controller.start_literal(
            editor.to_string().as_str(),
            "hello".to_string(),
            "hi".to_string(),
            true,
        );
        assert_eq!(start.total_matches, 2);
        assert!(controller.is_active());

        let progress = controller.accept_current(&mut editor).unwrap();
        assert_eq!(editor.to_string(), "hi world hello");
        assert_eq!(progress.replaced, 1);
        assert!(!progress.finished);
        assert_eq!(controller.current_range(), Some((9, 14)));

        let progress = controller.accept_current(&mut editor).unwrap();
        assert_eq!(editor.to_string(), "hi world hi");
        assert!(progress.finished);
    }

    #[test]
    fn literal_skip() {
        let mut editor = TextEditor::from_str("abc abc");
        let mut controller = QueryReplaceController::new();
        controller.start_literal(
            editor.to_string().as_str(),
            "abc".to_string(),
            "XYZ".to_string(),
            true,
        );
        let progress = controller.skip_current();
        assert_eq!(progress.skipped, 1);
        assert!(!progress.finished);
        assert_eq!(controller.current_range(), Some((4, 7)));
        let progress = controller.accept_current(&mut editor).unwrap();
        assert_eq!(progress.replaced, 1);
        assert!(progress.finished);
        assert_eq!(editor.to_string(), "abc XYZ");
    }

    #[test]
    fn cancel_restores_original_text() {
        let mut editor = TextEditor::from_str("one two one");
        let mut controller = QueryReplaceController::new();
        controller.start_literal(
            editor.to_string().as_str(),
            "one".to_string(),
            "1".to_string(),
            true,
        );
        controller.accept_current(&mut editor).unwrap();
        let summary = controller.cancel(&mut editor).unwrap();
        assert!(summary.cancelled);
        assert_eq!(editor.to_string(), "one two one");
        assert!(!controller.is_active());
    }

    #[test]
    fn highlights_generated() {
        let text = "abc\nabc";
        let mut controller = QueryReplaceController::new();
        controller.start_literal(text, "abc".to_string(), "x".to_string(), true);
        let highlights = controller.highlights(text);
        assert_eq!(highlights.len(), 2);
        assert_eq!(highlights[0].line, 0);
        assert_eq!(highlights[1].line, 1);
    }

    #[test]
    fn regex_replacement_basic() {
        let mut editor = TextEditor::from_str("name: John\nname: Alice");
        let mut controller = QueryReplaceController::new();
        let start = controller
            .start_regex(
                editor.to_string().as_str(),
                "name: (\\w+)".to_string(),
                "user: $1".to_string(),
                true,
            )
            .unwrap();
        assert_eq!(start.total_matches, 2);
        controller.accept_current(&mut editor).unwrap();
        controller.accept_current(&mut editor).unwrap();
        assert_eq!(editor.to_string(), "user: John\nuser: Alice");
    }
}
