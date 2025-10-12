# キーバインド設計仕様書

## 概要

本文書は、Altreテキストエディタのキーバインドシステムの設計仕様を定義する。Emacs風のキーバインドを基本としつつ、連続キー（C-x C-f等）の適切な処理と将来の拡張性を考慮した設計を行う。

## 設計目標

1. **Emacs互換性**: 基本的なEmacs風キーバインドの実装
2. **応答性**: キー入力に対する即座の反応（< 1ms）
3. **拡張性**: 将来のカスタマイズ機能への対応
4. **簡潔性**: MVPに必要な機能に集中したシンプルな実装

## キーバインド表現

### 基本キー表現

```rust
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
}
```

### キーシーケンス表現

```rust
/// キーシーケンス（連続キー対応）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence {
    pub keys: Vec<Key>,
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
        // "C-x C-f" -> [Ctrl+x, Ctrl+f]
        // 実装詳細は後述
    }
}
```

## MVP対象キーバインド

### 基本移動・編集

| キーバインド | 機能 | 実装優先度 |
|-------------|------|------------|
| `C-n` | 下移動 | 高 |
| `C-p` | 上移動 | 高 |
| `C-f` | 右移動 | 高 |
| `C-b` | 左移動 | 高 |
| `↑` | 上移動 | 高 |
| `↓` | 下移動 | 高 |
| `→` | 右移動 | 高 |
| `←` | 左移動 | 高 |
| `Backspace` | 前文字削除 | 高 |
| `Delete` | 後文字削除 | 高 |
| `Enter` | 改行 | 高 |
| `Tab` / `C-i` | タブ幅に沿ってインデント | 高 |
| `M-f` | 次の単語末尾へ移動 | 高 |
| `M-b` | 前の単語先頭へ移動 | 高 |
| `C-j` | 改行して既存インデントを適用 | 高 |
| `C-o` | カーソル位置に空行を挿入 | 高 |

### ファイル操作（C-xプレフィックス）

| キーバインド | 機能 | 実装優先度 |
|-------------|------|------------|
| `C-x C-f` | ファイルオープン | 高 |
| `C-x C-s` | ファイル保存 | 高 |
| `C-x C-c` | 終了 | 高 |

### コマンド実行

| キーバインド | 機能 | 実装優先度 |
|-------------|------|------------|
| `M-x` | コマンド実行 | 中 |

### プレフィックス操作

| キーバインド | 機能 | 実装優先度 |
|-------------|------|------------|
| `M-g g` | 指定行へ移動 | 高 |

## キーマップアーキテクチャ

### 混合階層設計

QA.mdの回答に基づき、基本は単一階層、C-xプレフィックスのみ特別扱いする混合アプローチを採用。

```rust
/// キーマップ構造
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// 単一キーのマッピング
    single_key_bindings: HashMap<Key, Action>,

    /// C-xプレフィックス用の特別マッピング
    cx_prefix_bindings: HashMap<Key, Action>,

    /// 部分マッチ状態の管理
    partial_match_state: PartialMatchState,
}

/// 部分マッチ状態
#[derive(Debug, Clone, PartialEq)]
enum PartialMatchState {
    /// マッチなし
    None,
    /// C-xプレフィックス待ち
    CxPrefix,
}

/// アクション定義
#[derive(Debug, Clone)]
pub enum Action {
    /// カーソル移動
    MoveCursor(Direction),
    /// 文字挿入
    InsertChar(char),
    /// 文字削除
    DeleteChar(DeleteDirection),
    /// 改行
    InsertNewline,
    /// タブ幅に沿ったインデント
    IndentForTab,
    /// 改行＋インデント
    NewlineAndIndent,
    /// 空行挿入
    OpenLine,
    /// ファイル操作
    FileOpen,
    FileSave,
    /// アプリケーション制御
    Quit,
    /// コマンド実行
    ExecuteCommand,
}
```

### キー処理フロー

```rust
impl KeyMap {
    /// キー入力を処理してアクションを返す
    pub fn process_key(&mut self, key: Key) -> KeyProcessResult {
        match self.partial_match_state {
            PartialMatchState::None => {
                self.process_initial_key(key)
            }
            PartialMatchState::CxPrefix => {
                self.process_cx_prefix_key(key)
            }
        }
    }

    fn process_initial_key(&mut self, key: Key) -> KeyProcessResult {
        // C-x の場合は部分マッチ状態に移行
        if key == Key::ctrl_x() {
            self.partial_match_state = PartialMatchState::CxPrefix;
            return KeyProcessResult::PartialMatch;
        }

        // 単一キーのマッピングを確認
        if let Some(action) = self.single_key_bindings.get(&key) {
            return KeyProcessResult::Action(action.clone());
        }

        // 通常文字の場合は挿入
        if let KeyCode::Char(ch) = key.code {
            if !key.modifiers.ctrl && !key.modifiers.alt {
                return KeyProcessResult::Action(Action::InsertChar(ch));
            }
        }

        // マッチしない場合はサイレント無視（QA.mdの回答）
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
}

/// キー処理結果
#[derive(Debug, Clone)]
pub enum KeyProcessResult {
    /// アクション実行
    Action(Action),
    /// 部分マッチ（連続キー待ち）
    PartialMatch,
    /// マッチなし（無視）
    NoMatch,
}
```

## crossterm統合

### キー正規化

```rust
use crossterm::event::{KeyEvent, KeyCode as CrosstermKeyCode, KeyModifiers as CrosstermModifiers};

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
            _ => {
                // 未対応のキーは無視
                return Key::unknown();
            }
        };

        Key { modifiers, code }
    }
}
```

### 特殊キー処理

```rust
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

    pub fn alt_x() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: false, alt: true, shift: false },
            code: KeyCode::Char('x'),
        }
    }

    /// 未知のキー（無視対象）
    fn unknown() -> Self {
        Self {
            modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
            code: KeyCode::Char('\0'),
        }
    }
}
```

## OS衝突回避戦略

QA.mdの回答に基づき、最小限の衝突回避を実装。

### 回避対象キー

| キー | 理由 | 対応方法 |
|------|------|----------|
| `Ctrl+C` | プロセス中断 | シグナルハンドリングで制御 |
| `Ctrl+Z` | プロセス一時停止 | raw mode で無効化 |
| `Ctrl+D` | EOF | 文脈に応じて処理 |

### 実装方針

```rust
/// OS衝突の回避
impl KeyMap {
    fn is_system_key(&self, key: &Key) -> bool {
        match (key.modifiers.ctrl, &key.code) {
            (true, KeyCode::Char('c')) => true,  // Ctrl+C
            (true, KeyCode::Char('z')) => true,  // Ctrl+Z
            _ => false,
        }
    }

    fn handle_system_key(&self, key: &Key) -> SystemKeyResult {
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
}
```

## エラーハンドリング

### キーパースエラー

```rust
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
```

### 不正入力処理

QA.mdの回答に基づき、サイレント無視を採用。

```rust
impl KeyMap {
    fn handle_unknown_key(&self, key: &Key) {
        // サイレント無視：何も表示しない
        // ログ記録も行わない（QA.mdの方針）
    }

    fn handle_partial_match_timeout(&mut self) {
        // 部分マッチ状態のタイムアウト処理
        self.partial_match_state = PartialMatchState::None;
        // 音声やメッセージなし
    }
}
```

## パフォーマンス考慮事項

### メモリ効率

```rust
/// キーマップの初期化
impl KeyMap {
    pub fn new() -> Self {
        // 事前に容量を確保してアロケーションを最小化
        let mut single_key_bindings = HashMap::with_capacity(32);
        let mut cx_prefix_bindings = HashMap::with_capacity(8);

        // MVPキーバインドの登録
        Self::register_mvp_bindings(&mut single_key_bindings, &mut cx_prefix_bindings);

        Self {
            single_key_bindings,
            cx_prefix_bindings,
            partial_match_state: PartialMatchState::None,
        }
    }

    fn register_mvp_bindings(
        single: &mut HashMap<Key, Action>,
        cx_prefix: &mut HashMap<Key, Action>
    ) {
        // 移動系
        single.insert(Key::ctrl_n(), Action::MoveCursor(Direction::Down));
        single.insert(Key::ctrl_p(), Action::MoveCursor(Direction::Up));
        single.insert(Key::ctrl_f(), Action::MoveCursor(Direction::Right));
        single.insert(Key::ctrl_b(), Action::MoveCursor(Direction::Left));

        // 矢印キー
        single.insert(Key::arrow_up(), Action::MoveCursor(Direction::Up));
        single.insert(Key::arrow_down(), Action::MoveCursor(Direction::Down));
        single.insert(Key::arrow_left(), Action::MoveCursor(Direction::Left));
        single.insert(Key::arrow_right(), Action::MoveCursor(Direction::Right));

        // 編集系
        single.insert(Key::backspace(), Action::DeleteChar(DeleteDirection::Backward));
        single.insert(Key::delete(), Action::DeleteChar(DeleteDirection::Forward));
        single.insert(Key::enter(), Action::InsertNewline);
        single.insert(
            Key {
                modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
                code: KeyCode::Tab,
            },
            Action::IndentForTab,
        );
        single.insert(
            Key {
                modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
                code: KeyCode::Char('j'),
            },
            Action::NewlineAndIndent,
        );
        single.insert(
            Key {
                modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
                code: KeyCode::Char('o'),
            },
            Action::OpenLine,
        );
        single.insert(
            Key {
                modifiers: KeyModifiers { ctrl: true, alt: false, shift: false },
                code: KeyCode::Char('i'),
            },
            Action::IndentForTab,
        );

        // ファイル操作（C-xプレフィックス）
        cx_prefix.insert(Key::ctrl_f(), Action::FileOpen);
        cx_prefix.insert(Key::ctrl_s(), Action::FileSave);
        cx_prefix.insert(Key::ctrl_c(), Action::Quit);

        // コマンド実行
        single.insert(Key::alt_x(), Action::ExecuteCommand);
    }
}
```

### 応答性能

```rust
impl KeyMap {
    /// 高速キールックアップ
    pub fn process_key_fast(&mut self, key: Key) -> KeyProcessResult {
        // ハッシュマップによるO(1)ルックアップ
        // 分岐を最小化した実装

        match self.partial_match_state {
            PartialMatchState::None => {
                // C-xチェック（最適化）
                if key.is_ctrl_x() {
                    self.partial_match_state = PartialMatchState::CxPrefix;
                    KeyProcessResult::PartialMatch
                } else if let Some(action) = self.single_key_bindings.get(&key) {
                    KeyProcessResult::Action(action.clone())
                } else if key.is_insertable_char() {
                    KeyProcessResult::Action(Action::InsertChar(key.to_char()))
                } else {
                    KeyProcessResult::NoMatch
                }
            }
            PartialMatchState::CxPrefix => {
                self.partial_match_state = PartialMatchState::None;
                self.cx_prefix_bindings.get(&key)
                    .map(|action| KeyProcessResult::Action(action.clone()))
                    .unwrap_or(KeyProcessResult::NoMatch)
            }
        }
    }
}
```

## 将来の拡張性

### カスタマイズ機能への準備

```rust
/// 設定可能なキーマップ
pub trait ConfigurableKeyMap {
    /// カスタムキーバインドの追加
    fn add_binding(&mut self, sequence: KeySequence, action: Action) -> Result<(), KeyMapError>;

    /// キーバインドの削除
    fn remove_binding(&mut self, sequence: &KeySequence) -> bool;

    /// 設定の保存/読み込み
    fn save_to_file(&self, path: &Path) -> Result<(), io::Error>;
    fn load_from_file(path: &Path) -> Result<Self, KeyMapError>;
}
```

### プラグインシステム連携

```rust
/// アクションの拡張
pub trait ActionHandler {
    fn handle_action(&mut self, action: &Action) -> ActionResult;
}

/// カスタムアクション
#[derive(Debug, Clone)]
pub enum CustomAction {
    /// 組み込みアクション
    Builtin(Action),
    /// プラグインアクション
    Plugin { plugin_id: String, action_name: String, params: Vec<String> },
}
```

## テスト戦略

### ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_key_binding() {
        let mut keymap = KeyMap::new();
        let result = keymap.process_key(Key::ctrl_n());
        assert_eq!(result, KeyProcessResult::Action(Action::MoveCursor(Direction::Down)));
    }

    #[test]
    fn test_cx_prefix_sequence() {
        let mut keymap = KeyMap::new();

        // C-x 入力
        let result1 = keymap.process_key(Key::ctrl_x());
        assert_eq!(result1, KeyProcessResult::PartialMatch);

        // C-f 入力
        let result2 = keymap.process_key(Key::ctrl_f());
        assert_eq!(result2, KeyProcessResult::Action(Action::FileOpen));
    }

    #[test]
    fn test_unknown_key_silence() {
        let mut keymap = KeyMap::new();
        let unknown_key = Key::ctrl_unknown();
        let result = keymap.process_key(unknown_key);
        assert_eq!(result, KeyProcessResult::NoMatch);
    }
}
```

### 統合テスト

```rust
#[test]
fn test_crossterm_integration() {
    use crossterm::event::KeyEvent;

    let crossterm_event = KeyEvent::new(
        crossterm::event::KeyCode::Char('x'),
        crossterm::event::KeyModifiers::CONTROL
    );

    let key: Key = crossterm_event.into();
    assert_eq!(key, Key::ctrl_x());
}
```

## 実装フェーズ

### Phase 1: 基本実装
1. `Key`、`KeySequence`、`KeyMap`の基本構造
2. 単一キーのマッピング
3. crossterm統合

### Phase 2: C-xプレフィックス
1. 部分マッチ状態管理
2. C-xプレフィックス処理
3. ファイル操作キーバインド

### Phase 3: 最適化・テスト
1. パフォーマンス最適化
2. 包括的テスト
3. ドキュメント整備

## 制限事項

### MVPでの制約
- モード（編集/コマンド等）は未サポート
- 複雑なキーシーケンス（3キー以上）は未サポート
- 動的なキーバインド変更は未サポート

### 既知の制限
- ターミナル固有のキーコード差異
- IME入力との併用時の制限
- プラットフォーム固有の修飾キー差異

これらの制限は将来バージョンで段階的に解決予定。
