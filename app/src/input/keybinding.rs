//! キーバインドシステム
//!
//! Emacs風キーバインドの処理とアクション実行を管理

use crossterm::event::{KeyCode as CrosstermKeyCode, KeyEvent, KeyModifiers as CrosstermModifiers};
use std::collections::HashMap;

/// キー入力の内部表現
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    /// 修飾キー
    pub modifiers: KeyModifiers,
    /// 基本キー
    pub code: KeyCode,
}

/// 修飾キーの組み合わせ
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// 基本キーコード
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Delete,
    Tab,
    Up,
    Down,
    Left,
    Right,
    F(u8),
    Esc,
    Unknown,
}

/// 旧インターフェースとの互換性用エイリアス
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombination {
    pub code: CrosstermKeyCode,
    pub modifiers: CrosstermModifiers,
}

impl Key {
    /// よく使用されるキーの定数
    pub fn ctrl_x() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('x'),
        }
    }

    pub fn ctrl_c() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('c'),
        }
    }

    pub fn ctrl_n() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('n'),
        }
    }

    pub fn ctrl_p() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('p'),
        }
    }

    pub fn ctrl_f() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('f'),
        }
    }

    pub fn ctrl_b() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('b'),
        }
    }

    pub fn ctrl_s() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
            code: KeyCode::Char('s'),
        }
    }

    pub fn alt_x() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: false, alt: true, shift: false },
            code: KeyCode::Char('x'),
        }
    }

    /// C-xキーかどうかを判定
    pub fn is_ctrl_x(&self) -> bool {
        self.modifiers.ctrl && !self.modifiers.alt && !self.modifiers.shift
            && matches!(self.code, KeyCode::Char('x'))
    }

    /// 挿入可能な文字かどうかを判定
    pub fn is_insertable_char(&self) -> bool {
        matches!(self.code, KeyCode::Char(_))
            && !self.modifiers.ctrl
            && !self.modifiers.alt
    }

    /// 文字に変換
    pub fn to_char(&self) -> char {
        match self.code {
            KeyCode::Char(c) => c,
            _ => '\0',
        }
    }
}

/// 旧インターフェースとの互換性
impl KeyCombination {
    pub fn new(code: CrosstermKeyCode, modifiers: CrosstermModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Ctrl+文字のキー組み合わせを作成
    pub fn ctrl(code: CrosstermKeyCode) -> Self {
        Self::new(code, CrosstermModifiers::CONTROL)
    }

    /// Alt+文字のキー組み合わせを作成
    pub fn alt(code: CrosstermKeyCode) -> Self {
        Self::new(code, CrosstermModifiers::ALT)
    }

    /// 修飾キーなしの文字
    pub fn plain(code: CrosstermKeyCode) -> Self {
        Self::new(code, CrosstermModifiers::NONE)
    }
}

/// アクション定義
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// カーソル移動
    MoveCursor(Direction),
    /// 文字挿入
    InsertChar(char),
    /// 文字削除
    DeleteChar(DeleteDirection),
    /// 改行
    InsertNewline,
    /// ファイル操作
    FileOpen,
    FileSave,
    /// アプリケーション制御
    Quit,
    /// コマンド実行
    ExecuteCommand,
}

/// 移動方向
#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// 削除方向
#[derive(Debug, Clone, PartialEq)]
pub enum DeleteDirection {
    Backward,  // Backspace
    Forward,   // Delete
}

/// キーシーケンス（連続キー対応）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence {
    pub keys: Vec<Key>,
}

/// 旧インターフェースとの互換性用
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LegacyKeySequence {
    keys: Vec<KeyCombination>,
}

impl KeySequence {
    /// 単一キーからシーケンスを作成
    pub fn single(key: Key) -> Self {
        Self { keys: vec![key] }
    }

    /// 複数キーからシーケンスを作成
    pub fn multi(keys: Vec<Key>) -> Self {
        Self { keys }
    }

    /// 文字列表現からパース
    pub fn parse(s: &str) -> Result<Self, KeyParseError> {
        if s.is_empty() {
            return Err(KeyParseError::EmptySequence);
        }

        let parts: Vec<&str> = s.split_whitespace().collect();
        let mut keys = Vec::new();

        for part in parts {
            let key = Self::parse_single_key(part)?;
            keys.push(key);
        }

        Ok(Self { keys })
    }

    fn parse_single_key(s: &str) -> Result<Key, KeyParseError> {
        let mut modifiers = KeyModifiers { ctrl: false, alt: false, shift: false };
        let mut remaining = s;

        // 修飾キーの解析
        loop {
            if remaining.starts_with("C-") {
                modifiers.ctrl = true;
                remaining = &remaining[2..];
            } else if remaining.starts_with("M-") {
                modifiers.alt = true;
                remaining = &remaining[2..];
            } else if remaining.starts_with("S-") {
                modifiers.shift = true;
                remaining = &remaining[2..];
            } else {
                break;
            }
        }

        let code = match remaining {
            "Enter" => KeyCode::Enter,
            "Backspace" => KeyCode::Backspace,
            "Delete" => KeyCode::Delete,
            "Tab" => KeyCode::Tab,
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Esc" => KeyCode::Esc,
            s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
            _ => return Err(KeyParseError::UnknownKey(remaining.to_string())),
        };

        Ok(Key { modifiers, code })
    }
}

/// 旧インターフェースとの互換性
impl LegacyKeySequence {
    pub fn new(keys: Vec<KeyCombination>) -> Self {
        Self { keys }
    }

    /// 単一キーからシーケンスを作成
    pub fn single(key: KeyCombination) -> Self {
        Self::new(vec![key])
    }

    /// シーケンスが完了しているか
    pub fn is_complete(&self) -> bool {
        !self.keys.is_empty()
    }

    /// シーケンスの長さ
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// シーケンスが空か
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// キーを追加
    pub fn push(&mut self, key: KeyCombination) {
        self.keys.push(key);
    }

    /// 前方一致チェック
    pub fn starts_with(&self, prefix: &LegacyKeySequence) -> bool {
        if prefix.len() > self.len() {
            return false;
        }
        self.keys[..prefix.len()] == prefix.keys
    }
}

/// crossterm統合
impl From<KeyEvent> for Key {
    fn from(event: KeyEvent) -> Self {
        let modifiers = KeyModifiers {
            ctrl: event.modifiers.contains(CrosstermModifiers::CONTROL),
            alt: event.modifiers.contains(CrosstermModifiers::ALT),
            shift: event.modifiers.contains(CrosstermModifiers::SHIFT),
        };

        let code = match event.code {
            CrosstermKeyCode::Char(c) => KeyCode::Char(c),
            CrosstermKeyCode::Enter => KeyCode::Enter,
            CrosstermKeyCode::Backspace => KeyCode::Backspace,
            CrosstermKeyCode::Delete => KeyCode::Delete,
            CrosstermKeyCode::Tab => KeyCode::Tab,
            CrosstermKeyCode::Up => KeyCode::Up,
            CrosstermKeyCode::Down => KeyCode::Down,
            CrosstermKeyCode::Left => KeyCode::Left,
            CrosstermKeyCode::Right => KeyCode::Right,
            CrosstermKeyCode::F(n) => KeyCode::F(n),
            CrosstermKeyCode::Esc => KeyCode::Esc,
            _ => KeyCode::Unknown,
        };

        Key { modifiers, code }
    }
}

impl ModernKeyMap {
    /// 新しいキーマップを作成
    pub fn new() -> Self {
        let mut single_key_bindings = HashMap::with_capacity(32);
        let mut cx_prefix_bindings = HashMap::with_capacity(8);

        Self::register_mvp_bindings(&mut single_key_bindings, &mut cx_prefix_bindings);

        Self {
            single_key_bindings,
            cx_prefix_bindings,
            partial_match_state: PartialMatchState::None,
        }
    }

    /// MVPキーバインドの登録
    fn register_mvp_bindings(
        single: &mut HashMap<Key, Action>,
        cx_prefix: &mut HashMap<Key, Action>,
    ) {
        // 移動系
        single.insert(Key::ctrl_n(), Action::MoveCursor(Direction::Down));
        single.insert(Key::ctrl_p(), Action::MoveCursor(Direction::Up));
        single.insert(Key::ctrl_f(), Action::MoveCursor(Direction::Right));
        single.insert(Key::ctrl_b(), Action::MoveCursor(Direction::Left));

        // 矢印キー
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Up }, Action::MoveCursor(Direction::Up));
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Down }, Action::MoveCursor(Direction::Down));
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Left }, Action::MoveCursor(Direction::Left));
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Right }, Action::MoveCursor(Direction::Right));

        // 編集系
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Backspace }, Action::DeleteChar(DeleteDirection::Backward));
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Delete }, Action::DeleteChar(DeleteDirection::Forward));
        single.insert(Key { modifiers: KeyModifiers { ctrl: false, alt: false, shift: false }, code: KeyCode::Enter }, Action::InsertNewline);

        // ファイル操作（C-xプレフィックス）
        cx_prefix.insert(Key::ctrl_f(), Action::FileOpen);
        cx_prefix.insert(Key::ctrl_s(), Action::FileSave);
        cx_prefix.insert(Key::ctrl_c(), Action::Quit);

        // コマンド実行
        single.insert(Key::alt_x(), Action::ExecuteCommand);
    }

    /// キー入力を処理してアクションを返す
    pub fn process_key(&mut self, key: Key) -> KeyProcessResult {
        // システムキーの処理
        match self.handle_system_key(&key) {
            SystemKeyResult::Cancel => return KeyProcessResult::NoMatch,
            SystemKeyResult::Ignore => return KeyProcessResult::NoMatch,
            SystemKeyResult::NotSystemKey => {}
        }

        match self.partial_match_state {
            PartialMatchState::None => self.process_initial_key(key),
            PartialMatchState::CxPrefix => self.process_cx_prefix_key(key),
        }
    }

    fn process_initial_key(&mut self, key: Key) -> KeyProcessResult {
        // C-x の場合は部分マッチ状態に移行
        if key.is_ctrl_x() {
            self.partial_match_state = PartialMatchState::CxPrefix;
            return KeyProcessResult::PartialMatch;
        }

        // 単一キーのマッピングを確認
        if let Some(action) = self.single_key_bindings.get(&key) {
            return KeyProcessResult::Action(action.clone());
        }

        // 通常文字の場合は挿入
        if key.is_insertable_char() {
            return KeyProcessResult::Action(Action::InsertChar(key.to_char()));
        }

        // マッチしない場合はサイレント無視
        KeyProcessResult::NoMatch
    }

    fn process_cx_prefix_key(&mut self, key: Key) -> KeyProcessResult {
        // 状態をリセット
        self.partial_match_state = PartialMatchState::None;

        // C-xプレフィックス用のマッピングを確認
        if let Some(action) = self.cx_prefix_bindings.get(&key) {
            return KeyProcessResult::Action(action.clone());
        }

        // マッチしない場合はサイレント無視
        KeyProcessResult::NoMatch
    }

    /// OS衝突の回避
    fn is_system_key(&self, key: &Key) -> bool {
        match (key.modifiers.ctrl, &key.code) {
            (true, KeyCode::Char('c')) => true,  // Ctrl+C
            (true, KeyCode::Char('z')) => true,  // Ctrl+Z
            _ => false,
        }
    }

    fn handle_system_key(&self, key: &Key) -> SystemKeyResult {
        if !self.is_system_key(key) {
            return SystemKeyResult::NotSystemKey;
        }

        match (key.modifiers.ctrl, &key.code) {
            (true, KeyCode::Char('c')) => {
                // 緊急終了ではなく、通常のキャンセル処理
                SystemKeyResult::Cancel
            }
            (true, KeyCode::Char('z')) => {
                // 一時停止は無視（ターミナルのraw modeで制御）
                SystemKeyResult::Ignore
            }
            _ => SystemKeyResult::NotSystemKey,
        }
    }

    /// 部分マッチ状態のリセット
    pub fn reset_partial_match(&mut self) {
        self.partial_match_state = PartialMatchState::None;
    }

    /// 現在の部分マッチ状態を取得
    pub fn is_partial_match(&self) -> bool {
        !matches!(self.partial_match_state, PartialMatchState::None)
    }
}

impl Default for ModernKeyMap {
    fn default() -> Self {
        Self::new()
    }
}

/// キーパースエラー
#[derive(Debug, thiserror::Error)]
pub enum KeyParseError {
    #[error("Invalid key sequence format: {0}")]
    InvalidFormat(String),

    #[error("Unknown modifier: {0}")]
    UnknownModifier(String),

    #[error("Unknown key: {0}")]
    UnknownKey(String),

    #[error("Empty key sequence")]
    EmptySequence,
}

/// キーマップエラー
#[derive(Debug, thiserror::Error)]
pub enum KeyMapError {
    #[error("Key binding conflict: {0:?}")]
    Conflict(KeySequence),

    #[error("Invalid key sequence: {0}")]
    InvalidSequence(#[from] KeyParseError),
}

/// キー処理結果
#[derive(Debug, Clone, PartialEq)]
pub enum KeyProcessResult {
    /// アクション実行
    Action(Action),
    /// 部分マッチ（連続キー待ち）
    PartialMatch,
    /// マッチなし（無視）
    NoMatch,
}

/// システムキー処理結果
#[derive(Debug, Clone, PartialEq)]
pub enum SystemKeyResult {
    /// キャンセル処理
    Cancel,
    /// 無視
    Ignore,
    /// システムキーではない
    NotSystemKey,
}

/// キーバインドの種類（旧インターフェース）
#[derive(Debug, Clone)]
pub enum KeyBinding {
    /// コマンドへのバインド
    Command(String),
    /// プレフィックスキー（複数キー入力の開始）
    Prefix,
}

/// 部分マッチ状態
#[derive(Debug, Clone, PartialEq)]
enum PartialMatchState {
    /// マッチなし
    None,
    /// C-xプレフィックス待ち
    CxPrefix,
}

/// キーマップ構造
#[derive(Debug, Clone)]
pub struct ModernKeyMap {
    /// 単一キーのマッピング
    single_key_bindings: HashMap<Key, Action>,

    /// C-xプレフィックス用の特別マッピング
    cx_prefix_bindings: HashMap<Key, Action>,

    /// 部分マッチ状態の管理
    partial_match_state: PartialMatchState,
}

/// キーマップ管理（旧インターフェース）
#[derive(Debug)]
pub struct KeyMap {
    /// グローバルキーバインド
    global_bindings: HashMap<LegacyKeySequence, KeyBinding>,
    /// 現在の入力シーケンス
    current_sequence: LegacyKeySequence,
}

impl KeyMap {
    pub fn new() -> Self {
        let mut keymap = Self {
            global_bindings: HashMap::new(),
            current_sequence: LegacyKeySequence::new(vec![]),
        };
        keymap.initialize_default_bindings();
        keymap
    }

    /// デフォルトのEmacsスタイルキーバインドを設定
    fn initialize_default_bindings(&mut self) {
        // 基本移動
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::ctrl(CrosstermKeyCode::Char('f'))),
            KeyBinding::Command("forward-char".to_string()),
        );
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::ctrl(CrosstermKeyCode::Char('b'))),
            KeyBinding::Command("backward-char".to_string()),
        );
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::ctrl(CrosstermKeyCode::Char('n'))),
            KeyBinding::Command("next-line".to_string()),
        );
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::ctrl(CrosstermKeyCode::Char('p'))),
            KeyBinding::Command("previous-line".to_string()),
        );

        // 削除
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::plain(CrosstermKeyCode::Backspace)),
            KeyBinding::Command("delete-backward-char".to_string()),
        );
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::ctrl(CrosstermKeyCode::Char('d'))),
            KeyBinding::Command("delete-char".to_string()),
        );

        // ファイル操作プレフィックス
        self.bind_global(
            LegacyKeySequence::single(KeyCombination::ctrl(CrosstermKeyCode::Char('x'))),
            KeyBinding::Prefix,
        );

        // C-x C-f (find-file)
        self.bind_global(
            LegacyKeySequence::new(vec![
                KeyCombination::ctrl(CrosstermKeyCode::Char('x')),
                KeyCombination::ctrl(CrosstermKeyCode::Char('f')),
            ]),
            KeyBinding::Command("find-file".to_string()),
        );

        // C-x C-s (save-buffer)
        self.bind_global(
            LegacyKeySequence::new(vec![
                KeyCombination::ctrl(CrosstermKeyCode::Char('x')),
                KeyCombination::ctrl(CrosstermKeyCode::Char('s')),
            ]),
            KeyBinding::Command("save-buffer".to_string()),
        );

        // C-x C-c (save-buffers-kill-terminal)
        self.bind_global(
            LegacyKeySequence::new(vec![
                KeyCombination::ctrl(CrosstermKeyCode::Char('x')),
                KeyCombination::ctrl(CrosstermKeyCode::Char('c')),
            ]),
            KeyBinding::Command("save-buffers-kill-terminal".to_string()),
        );
    }

    /// グローバルキーバインドを追加
    pub fn bind_global(&mut self, sequence: LegacyKeySequence, binding: KeyBinding) {
        self.global_bindings.insert(sequence, binding);
    }

    /// キー入力を処理し、コマンドまたは状態を返す
    pub fn process_key(&mut self, key_event: KeyEvent) -> KeyLookupResult {
        let key_combo = KeyCombination::new(key_event.code, key_event.modifiers);
        self.current_sequence.push(key_combo);

        // 完全一致をチェック
        if let Some(binding) = self.global_bindings.get(&self.current_sequence) {
            match binding {
                KeyBinding::Command(cmd) => {
                    let result = KeyLookupResult::Command(cmd.clone());
                    self.current_sequence = LegacyKeySequence::new(vec![]);
                    return result;
                }
                KeyBinding::Prefix => {
                    return KeyLookupResult::Prefix;
                }
            }
        }

        // 前方一致をチェック（プレフィックスの可能性）
        let has_prefix = self.global_bindings.keys()
            .any(|seq| seq.starts_with(&self.current_sequence));

        if has_prefix {
            KeyLookupResult::Prefix
        } else {
            // 一致しない場合はシーケンスをリセット
            self.current_sequence = LegacyKeySequence::new(vec![]);
            KeyLookupResult::Unbound
        }
    }

    /// 現在のキーシーケンスをリセット
    pub fn reset_sequence(&mut self) {
        self.current_sequence = LegacyKeySequence::new(vec![]);
    }

    /// 現在のキーシーケンスを取得
    pub fn current_sequence(&self) -> &LegacyKeySequence {
        &self.current_sequence
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::new()
    }
}

/// キー検索の結果
#[derive(Debug, Clone)]
pub enum KeyLookupResult {
    /// コマンドが見つかった
    Command(String),
    /// プレフィックスキー（続きの入力待ち）
    Prefix,
    /// バインドされていない
    Unbound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_combination() {
        let ctrl_f = KeyCombination::ctrl(CrosstermKeyCode::Char('f'));
        assert_eq!(ctrl_f.code, CrosstermKeyCode::Char('f'));
        assert_eq!(ctrl_f.modifiers, CrosstermModifiers::CONTROL);
    }

    #[test]
    fn test_key_sequence() {
        let mut seq = LegacyKeySequence::new(vec![]);
        assert!(seq.is_empty());

        seq.push(KeyCombination::ctrl(CrosstermKeyCode::Char('x')));
        assert_eq!(seq.len(), 1);
        assert!(!seq.is_empty());
    }

    #[test]
    fn test_keymap_basic_commands() {
        let mut keymap = KeyMap::new();

        // C-f should map to forward-char
        let key_event = KeyEvent::new(CrosstermKeyCode::Char('f'), CrosstermModifiers::CONTROL);
        let result = keymap.process_key(key_event);

        match result {
            KeyLookupResult::Command(cmd) => assert_eq!(cmd, "forward-char"),
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_keymap_prefix_sequence() {
        let mut keymap = KeyMap::new();

        // C-x should be a prefix
        let key_event = KeyEvent::new(CrosstermKeyCode::Char('x'), CrosstermModifiers::CONTROL);
        let result = keymap.process_key(key_event);

        match result {
            KeyLookupResult::Prefix => {},
            _ => panic!("Expected prefix"),
        }

        // C-x C-f should map to find-file
        let key_event = KeyEvent::new(CrosstermKeyCode::Char('f'), CrosstermModifiers::CONTROL);
        let result = keymap.process_key(key_event);

        match result {
            KeyLookupResult::Command(cmd) => assert_eq!(cmd, "find-file"),
            _ => panic!("Expected command"),
        }
    }

    // 新しいAPIのテスト
    #[test]
    fn test_modern_keymap_single_key() {
        let mut keymap = ModernKeyMap::new();
        let result = keymap.process_key(Key::ctrl_n());
        assert_eq!(result, KeyProcessResult::Action(Action::MoveCursor(Direction::Down)));
    }

    #[test]
    fn test_modern_keymap_cx_prefix() {
        let mut keymap = ModernKeyMap::new();

        // C-x 入力
        let result1 = keymap.process_key(Key::ctrl_x());
        assert_eq!(result1, KeyProcessResult::PartialMatch);

        // C-f 入力
        let result2 = keymap.process_key(Key::ctrl_f());
        assert_eq!(result2, KeyProcessResult::Action(Action::FileOpen));
    }

    #[test]
    fn test_modern_keymap_insertable_char() {
        let mut keymap = ModernKeyMap::new();
        let key = Key {
            modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
            code: KeyCode::Char('a'),
        };
        let result = keymap.process_key(key);
        assert_eq!(result, KeyProcessResult::Action(Action::InsertChar('a')));
    }

    #[test]
    fn test_crossterm_integration() {
        let crossterm_event = KeyEvent::new(
            CrosstermKeyCode::Char('x'),
            CrosstermModifiers::CONTROL
        );

        let key: Key = crossterm_event.into();
        assert_eq!(key, Key::ctrl_x());
    }

    #[test]
    fn test_key_sequence_parse() {
        let sequence = KeySequence::parse("C-x C-f").unwrap();
        assert_eq!(sequence.keys.len(), 2);
        assert_eq!(sequence.keys[0], Key::ctrl_x());
        assert_eq!(sequence.keys[1], Key::ctrl_f());
    }
}