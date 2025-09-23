//! GapBuffer public API property tests
//!
//! These complement the module-level invariants by exercising only the exposed
//! methods so downstream integrations can rely on stable behaviour.

use altre::buffer::GapBuffer;
use proptest::{prelude::*, prop_oneof};
use proptest::test_runner::Config as ProptestConfig;

#[derive(Debug, Clone)]
enum Operation {
    InsertChar { pos: usize, ch: char },
    InsertStr { pos: usize, text: String },
    Delete { pos: usize },
}

fn small_unicode_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..48)
        .prop_map(|chars| chars.into_iter().collect::<String>())
}

fn operation_strategy() -> impl Strategy<Value = Operation> {
    let insert_char = (0u16..192u16, any::<char>())
        .prop_map(|(pos, ch)| Operation::InsertChar { pos: pos as usize, ch });
    let insert_str = (0u16..192u16, proptest::collection::vec(any::<char>(), 0..5))
        .prop_map(|(pos, chars)| Operation::InsertStr {
            pos: pos as usize,
            text: chars.into_iter().collect(),
        });
    let delete = (0u16..192u16).prop_map(|pos| Operation::Delete { pos: pos as usize });

    prop_oneof![insert_char, insert_str, delete]
}

fn char_to_byte_index(s: &str, char_pos: usize) -> usize {
    if char_pos >= s.chars().count() {
        return s.len();
    }
    s.char_indices()
        .nth(char_pos)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn gap_buffer_public_operations_match_string_model(
        initial in small_unicode_string(),
        ops in proptest::collection::vec(operation_strategy(), 0..20)
    ) {
        let mut buffer = GapBuffer::from_str(&initial);
        let mut model = initial.clone();

        for op in ops {
            match op {
                Operation::InsertChar { pos, ch } => {
                    let insert_pos = pos.min(buffer.len_chars());
                    buffer.insert(insert_pos, ch).unwrap();
                    let byte_idx = char_to_byte_index(&model, insert_pos);
                    model.insert(byte_idx, ch);
                }
                Operation::InsertStr { pos, text } => {
                    if text.is_empty() {
                        continue;
                    }
                    let insert_pos = pos.min(buffer.len_chars());
                    buffer.insert_str(insert_pos, &text).unwrap();
                    let byte_idx = char_to_byte_index(&model, insert_pos);
                    model.insert_str(byte_idx, &text);
                }
                Operation::Delete { pos } => {
                    if buffer.len_chars() == 0 {
                        continue;
                    }
                    let delete_pos = pos % buffer.len_chars();
                    let expected_char = model.chars().nth(delete_pos).unwrap();
                    let deleted = buffer.delete(delete_pos).unwrap();
                    prop_assert_eq!(deleted, expected_char);
                    let start = char_to_byte_index(&model, delete_pos);
                    let end = char_to_byte_index(&model, delete_pos + 1);
                    model.replace_range(start..end, "");
                }
            }
        }

        prop_assert_eq!(buffer.to_string(), model);
    }
}
