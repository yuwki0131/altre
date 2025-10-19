use std::cell::RefCell;
use std::rc::Rc;

use crate::buffer::editor::EditOperations;
use crate::buffer::{ChangeEvent, ChangeListener, CursorPosition, TextEditor};
use crate::error::Result;

/// コマンド種別（履歴管理用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryCommandKind {
    InsertChar,
    DeleteBackward,
    Other,
}

/// 編集履歴スタック
#[derive(Debug, Clone, Default)]
pub struct HistoryStack {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl HistoryStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn push(&mut self, mut entry: HistoryEntry) {
        if let Some(last) = self.undo.last_mut() {
            if last.try_merge_with(&entry) {
                last.merge_with(entry);
                self.redo.clear();
                return;
            }
        }
        entry.compact();
        self.undo.push(entry);
        self.redo.clear();
    }

    pub fn take_for_undo(&mut self) -> Option<HistoryEntry> {
        self.undo.pop()
    }

    pub fn push_redo(&mut self, entry: HistoryEntry) {
        self.redo.push(entry);
    }

    pub fn take_for_redo(&mut self) -> Option<HistoryEntry> {
        self.redo.pop()
    }

    pub fn push_without_clearing(&mut self, entry: HistoryEntry) {
        self.undo.push(entry);
    }
}

/// 履歴エントリ
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command_kind: HistoryCommandKind,
    pub operations: Vec<AtomicEdit>,
    pub cursor_before: CursorSnapshot,
    pub cursor_after: CursorSnapshot,
}

impl HistoryEntry {
    fn try_merge_with(&self, other: &HistoryEntry) -> bool {
        match (self.command_kind, other.command_kind) {
            (HistoryCommandKind::InsertChar, HistoryCommandKind::InsertChar) => {
                self.is_simple_insert() && other.is_simple_insert() && can_merge_insert(self, other)
            }
            (HistoryCommandKind::DeleteBackward, HistoryCommandKind::DeleteBackward) => {
                self.is_simple_delete() && other.is_simple_delete() && can_merge_delete(self, other)
            }
            _ => false,
        }
    }

    fn merge_with(&mut self, other: HistoryEntry) {
        match (self.command_kind, other.command_kind) {
            (HistoryCommandKind::InsertChar, HistoryCommandKind::InsertChar) => {
                let prev_op = self.operations.first_mut().expect("insert op");
                let other_op = other.operations.first().expect("insert op");
                if let (
                    AtomicEdit::Insert {
                        text: prev_text, ..
                    },
                    AtomicEdit::Insert {
                        text: other_text, ..
                    },
                ) = (prev_op, other_op)
                {
                    prev_text.push_str(other_text);
                }
                self.cursor_after = other.cursor_after;
            }
            (HistoryCommandKind::DeleteBackward, HistoryCommandKind::DeleteBackward) => {
                let prev_op = self.operations.first_mut().expect("delete op");
                let other_op = other.operations.first().expect("delete op");
                if let (
                    AtomicEdit::Delete {
                        position: prev_pos,
                        text: prev_text,
                    },
                    AtomicEdit::Delete {
                        position: other_pos,
                        text: other_text,
                    },
                ) = (prev_op, other_op)
                {
                    *prev_pos = *other_pos;
                    let mut combined = other_text.clone();
                    combined.push_str(prev_text);
                    *prev_text = combined;
                }
                self.cursor_after = other.cursor_after;
            }
            _ => unreachable!(),
        }
    }

    fn is_simple_insert(&self) -> bool {
        matches!(self.operations.as_slice(), [AtomicEdit::Insert { .. }])
    }

    fn is_simple_delete(&self) -> bool {
        matches!(self.operations.as_slice(), [AtomicEdit::Delete { .. }])
    }

    fn compact(&mut self) {
        // 将来のための拡張ポイント。現状は何もしない。
    }
}

fn can_merge_insert(prev: &HistoryEntry, new: &HistoryEntry) -> bool {
    let (prev_pos, prev_text) = match prev.operations.first() {
        Some(AtomicEdit::Insert { position, text }) => (*position, text),
        _ => return false,
    };
    let (new_pos, new_text) = match new.operations.first() {
        Some(AtomicEdit::Insert { position, text }) => (*position, text),
        _ => return false,
    };

    if !is_word_text(prev_text) || !is_word_text(new_text) {
        return false;
    }

    let prev_len = prev_text.chars().count();
    if new_text.is_empty() {
        return false;
    }

    if new_pos != prev_pos + prev_len {
        return false;
    }

    true
}

fn can_merge_delete(prev: &HistoryEntry, new: &HistoryEntry) -> bool {
    let (prev_pos, prev_text) = match prev.operations.first() {
        Some(AtomicEdit::Delete { position, text }) => (*position, text),
        _ => return false,
    };
    let (new_pos, new_text) = match new.operations.first() {
        Some(AtomicEdit::Delete { position, text }) => (*position, text),
        _ => return false,
    };

    if !is_word_text(prev_text) || !is_word_text(new_text) {
        return false;
    }

    if new_text.is_empty() {
        return false;
    }

    if new_pos + new_text.chars().count() != prev_pos {
        return false;
    }

    true
}

fn is_word_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// 履歴操作の最小単位
#[derive(Debug, Clone)]
pub enum AtomicEdit {
    Insert { position: usize, text: String },
    Delete { position: usize, text: String },
}

/// カーソルスナップショット
#[derive(Debug, Clone)]
pub struct CursorSnapshot {
    pub position: CursorPosition,
    pub mark: Option<usize>,
}

impl CursorSnapshot {
    pub fn from_editor(editor: &TextEditor) -> Self {
        let position = *editor.cursor();
        let mark = editor.mark();
        Self { position, mark }
    }
}

/// 履歴記録器
#[derive(Clone)]
pub struct HistoryRecorder {
    inner: Rc<RefCell<HistoryRecorderState>>,
}

impl HistoryRecorder {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(HistoryRecorderState::new())),
        }
    }

    pub fn begin_command(&self, kind: HistoryCommandKind, cursor: CursorSnapshot) {
        self.inner.borrow_mut().begin_command(kind, cursor);
    }

    pub fn end_command(&self, cursor_after: CursorSnapshot, success: bool) -> Option<HistoryEntry> {
        self.inner.borrow_mut().end_command(cursor_after, success)
    }

    pub fn suspend(&self, suspended: bool) {
        self.inner.borrow_mut().suspended = suspended;
    }

    pub fn reset(&self) {
        self.inner.borrow_mut().reset();
    }
}

impl ChangeListener for HistoryRecorder {
    fn on_change(&mut self, event: &ChangeEvent) {
        self.inner.borrow_mut().on_change(event);
    }
}

#[derive(Debug)]
struct HistoryRecorderState {
    recording: bool,
    suspended: bool,
    command_kind: HistoryCommandKind,
    cursor_before: Option<CursorSnapshot>,
    operations: Vec<AtomicEdit>,
}

impl HistoryRecorderState {
    fn new() -> Self {
        Self {
            recording: false,
            suspended: false,
            command_kind: HistoryCommandKind::Other,
            cursor_before: None,
            operations: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.recording = false;
        self.command_kind = HistoryCommandKind::Other;
        self.cursor_before = None;
        self.operations.clear();
    }

    fn begin_command(&mut self, kind: HistoryCommandKind, cursor: CursorSnapshot) {
        self.recording = true;
        self.command_kind = kind;
        self.cursor_before = Some(cursor);
        self.operations.clear();
    }

    fn end_command(&mut self, cursor_after: CursorSnapshot, success: bool) -> Option<HistoryEntry> {
        if !self.recording {
            return None;
        }

        let operations = std::mem::take(&mut self.operations);
        let cursor_before = self.cursor_before.take();
        self.recording = false;

        if !success || operations.is_empty() {
            return None;
        }

        Some(HistoryEntry {
            command_kind: self.command_kind,
            operations,
            cursor_before: cursor_before.expect("cursor before"),
            cursor_after,
        })
    }

    fn on_change(&mut self, event: &ChangeEvent) {
        if !self.recording || self.suspended {
            return;
        }

        match event {
            ChangeEvent::Insert { position, content } => {
                if !content.is_empty() {
                    self.operations.push(AtomicEdit::Insert {
                        position: *position,
                        text: content.clone(),
                    });
                }
            }
            ChangeEvent::Delete { position, content } => {
                if !content.is_empty() {
                    self.operations.push(AtomicEdit::Delete {
                        position: *position,
                        text: content.clone(),
                    });
                }
            }
            ChangeEvent::CursorMove { .. } => {}
        }
    }
}

/// 履歴管理マネージャ
pub struct HistoryManager {
    stack: HistoryStack,
    recorder: HistoryRecorder,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            stack: HistoryStack::new(),
            recorder: HistoryRecorder::new(),
        }
    }

    pub fn bind_editor(&mut self, editor: &mut TextEditor) {
        self.recorder.reset();
        editor.add_change_listener(Box::new(self.recorder.clone()));
    }

    pub fn replace_stack(&mut self, stack: HistoryStack, editor: &mut TextEditor) {
        self.stack = stack;
        self.bind_editor(editor);
    }

    pub fn stack(&self) -> &HistoryStack {
        &self.stack
    }

    pub fn stack_mut(&mut self) -> &mut HistoryStack {
        &mut self.stack
    }

    pub fn begin_command(&self, kind: HistoryCommandKind, editor: &TextEditor) {
        let snapshot = CursorSnapshot::from_editor(editor);
        self.recorder.begin_command(kind, snapshot);
    }

    pub fn end_command(&mut self, editor: &TextEditor, success: bool) {
        let snapshot = CursorSnapshot::from_editor(editor);
        if let Some(entry) = self.recorder.end_command(snapshot, success) {
            self.stack.push(entry);
        }
    }

    pub fn undo(&mut self, editor: &mut TextEditor) -> Result<bool> {
        let Some(entry) = self.stack.take_for_undo() else {
            return Ok(false);
        };
        self.recorder.suspend(true);
        let result = apply_undo(editor, &entry);
        self.recorder.suspend(false);
        match result {
            Ok(_) => {
                self.stack.push_redo(entry);
                Ok(true)
            }
            Err(err) => {
                self.stack.push_without_clearing(entry);
                Err(err)
            }
        }
    }

    pub fn redo(&mut self, editor: &mut TextEditor) -> Result<bool> {
        let Some(entry) = self.stack.take_for_redo() else {
            return Ok(false);
        };
        self.recorder.suspend(true);
        let result = apply_redo(editor, &entry);
        self.recorder.suspend(false);
        match result {
            Ok(_) => {
                self.stack.push_without_clearing(entry);
                Ok(true)
            }
            Err(err) => {
                self.stack.push_redo(entry);
                Err(err)
            }
        }
    }
}

fn apply_undo(editor: &mut TextEditor, entry: &HistoryEntry) -> Result<()> {
    for op in entry.operations.iter().rev() {
        match op {
            AtomicEdit::Insert { position, text } => {
                let start = *position;
                let end = start + text.chars().count();
                editor.delete_range(start, end)?;
            }
            AtomicEdit::Delete { position, text } => {
                editor.move_cursor_to_char(*position)?;
                editor.insert_str(text)?;
            }
        }
    }
    editor.set_cursor(entry.cursor_before.position);
    Ok(())
}

fn apply_redo(editor: &mut TextEditor, entry: &HistoryEntry) -> Result<()> {
    for op in entry.operations.iter() {
        match op {
            AtomicEdit::Insert { position, text } => {
                editor.move_cursor_to_char(*position)?;
                editor.insert_str(text)?;
            }
            AtomicEdit::Delete { position, text } => {
                let start = *position;
                let end = start + text.chars().count();
                editor.delete_range(start, end)?;
            }
        }
    }
    editor.set_cursor(entry.cursor_after.position);
    Ok(())
}
