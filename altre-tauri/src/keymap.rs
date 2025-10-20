use altre::error::{AltreError, Result};
use crossterm::event::{KeyCode as CrosstermKeyCode, KeyEvent, KeyModifiers as CrosstermModifiers};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyStrokePayload {
    pub key: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct KeySequencePayload {
    /// 入力されたキー列。各要素は時系列順のチャンク（例: `["C-x"]`, `["C-f"]`）を表す。
    #[serde(default)]
    pub sequence: Vec<Vec<KeyStrokePayload>>,
}

#[derive(Debug, Error)]
pub enum KeyConversionError {
    #[error("未対応のキーです: {0}")]
    UnsupportedKey(String),
}

impl KeySequencePayload {
    pub fn from_strokes(strokes: Vec<KeyStrokePayload>) -> Self {
        Self {
            sequence: strokes.into_iter().map(|stroke| vec![stroke]).collect(),
        }
    }

    pub fn from_sequence(sequence: Vec<Vec<KeyStrokePayload>>) -> Self {
        Self { sequence }
    }

    pub fn into_key_events(self) -> Result<Vec<KeyEvent>> {
        let mut events = Vec::new();
        for chunk in self.sequence {
            for stroke in chunk {
                let event = stroke.to_key_event().map_err(|err| {
                    AltreError::Application(format!("キー入力の変換に失敗しました: {err}"))
                })?;
                events.push(event);
            }
        }
        Ok(events)
    }
}

impl KeyStrokePayload {
    pub fn to_key_event(self) -> std::result::Result<KeyEvent, KeyConversionError> {
        let code = parse_key_code(&self.key)?;
        let mut modifiers = CrosstermModifiers::empty();
        if self.ctrl {
            modifiers |= CrosstermModifiers::CONTROL;
        }
        if self.alt {
            modifiers |= CrosstermModifiers::ALT;
        }
        if self.shift {
            modifiers |= CrosstermModifiers::SHIFT;
        }
        Ok(KeyEvent::new(code, modifiers))
    }
}

fn parse_key_code(raw: &str) -> std::result::Result<CrosstermKeyCode, KeyConversionError> {
    let key = raw.trim();
    if key.len() == 1 {
        let ch = key.chars().next().unwrap();
        return Ok(CrosstermKeyCode::Char(ch));
    }

    let code = match key.to_ascii_lowercase().as_str() {
        "enter" => CrosstermKeyCode::Enter,
        "backspace" => CrosstermKeyCode::Backspace,
        "delete" => CrosstermKeyCode::Delete,
        "tab" => CrosstermKeyCode::Tab,
        "escape" | "esc" => CrosstermKeyCode::Esc,
        "up" => CrosstermKeyCode::Up,
        "down" => CrosstermKeyCode::Down,
        "left" => CrosstermKeyCode::Left,
        "right" => CrosstermKeyCode::Right,
        _ => return Err(KeyConversionError::UnsupportedKey(key.to_string())),
    };

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_simple_key() {
        let payload = KeyStrokePayload {
            key: "a".into(),
            ctrl: false,
            alt: false,
            shift: false,
        };
        let event = payload.to_key_event().unwrap();
        assert_eq!(event.code, CrosstermKeyCode::Char('a'));
        assert!(event.modifiers.is_empty());
    }

    #[test]
    fn converts_ctrl_key() {
        let payload = KeyStrokePayload {
            key: "x".into(),
            ctrl: true,
            alt: false,
            shift: false,
        };
        let event = payload.to_key_event().unwrap();
        assert_eq!(event.code, CrosstermKeyCode::Char('x'));
        assert!(event.modifiers.contains(CrosstermModifiers::CONTROL));
    }

    #[test]
    fn rejects_unknown_key() {
        let payload = KeyStrokePayload {
            key: "F13".into(),
            ctrl: false,
            alt: false,
            shift: false,
        };
        assert!(payload.to_key_event().is_err());
    }

    #[test]
    fn converts_sequence_payload() {
        let payload = KeySequencePayload::from_sequence(vec![
            vec![KeyStrokePayload {
                key: "x".into(),
                ctrl: true,
                alt: false,
                shift: false,
            }],
            vec![KeyStrokePayload {
                key: "f".into(),
                ctrl: true,
                alt: false,
                shift: false,
            }],
        ]);

        let events = payload
            .into_key_events()
            .expect("キー列の変換に失敗しました");
        assert_eq!(events.len(), 2);
        assert!(events[0].modifiers.contains(CrosstermModifiers::CONTROL));
        assert_eq!(events[1].code, CrosstermKeyCode::Char('f'));
    }
}
