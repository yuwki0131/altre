# ミニバッファテスト仕様

## 概要

ミニバッファシステムの品質保証のためのテスト仕様書。単体テスト、統合テスト、プロパティベーステストを組み合わせて信頼性を確保する。

## テスト対象

### 1. ミニバッファコア機能
- プロンプト表示・管理
- 文字列入力処理とUTF-8対応
- 履歴管理（セッション内）
- 入力キャンセル（C-g）
- メッセージ表示と自動消去

### 2. ファイル操作コマンド
- `C-x C-f` (find-file) - ファイルを開く
- `C-x C-s` (save-buffer) - ファイル保存
- `C-x C-c` (save-buffers-kill-terminal) - 終了

### 3. 補完機能
- ファイルパス補完（50候補制限）
- TABでの補完実行
- 候補表示とナビゲーション
- 部分一致での絞り込み

### 4. エラーハンドリング
- ファイル読み込みエラー
- 権限エラー
- パス不正エラー
- ユーザーフレンドリーなメッセージ表示

## 単体テスト

### 文字入力処理テスト (`test_input_processing`)

```rust
#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn test_char_insertion() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(None);

        // ASCII文字
        assert_eq!(minibuffer.handle_key(Key::char('a')), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().input, "a");
        assert_eq!(minibuffer.state().cursor_pos, 1);

        // Unicode文字
        assert_eq!(minibuffer.handle_key(Key::char('あ')), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().input, "aあ");
        assert_eq!(minibuffer.state().cursor_pos, 2);
    }

    #[test]
    fn test_backspace_deletion() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("test"));

        let key = Key { code: KeyCode::Backspace, modifiers: KeyModifiers::default() };
        assert_eq!(minibuffer.handle_key(key), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().input, "tes");
        assert_eq!(minibuffer.state().cursor_pos, 3);
    }

    #[test]
    fn test_cursor_movement() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("test"));

        // Home
        let home_key = Key {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false }
        };
        assert_eq!(minibuffer.handle_key(home_key), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().cursor_pos, 0);

        // End
        let end_key = Key {
            code: KeyCode::Char('e'),
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false }
        };
        assert_eq!(minibuffer.handle_key(end_key), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().cursor_pos, 4);
    }
}
```

### 履歴機能テスト (`test_history_management`)

```rust
#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn test_history_addition() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("file1.txt"));

        let enter_key = Key { code: KeyCode::Enter, modifiers: KeyModifiers::default() };
        assert_eq!(minibuffer.handle_key(enter_key), MinibufferResult::Execute("find-file file1.txt".to_string()));

        minibuffer.start_find_file(Some("file2.txt"));
        assert_eq!(minibuffer.handle_key(enter_key), MinibufferResult::Execute("find-file file2.txt".to_string()));

        // 履歴確認
        assert_eq!(minibuffer.state().history.len(), 2);
        assert_eq!(minibuffer.state().history.get_entry(0), Some(&"file2.txt".to_string()));
        assert_eq!(minibuffer.state().history.get_entry(1), Some(&"file1.txt".to_string()));
    }

    #[test]
    fn test_history_navigation() {
        let mut minibuffer = ModernMinibuffer::new();

        // 履歴にアイテムを追加
        minibuffer.state.history.add_entry("old_command".to_string());
        minibuffer.start_execute_command();

        // 履歴前へ
        let ctrl_p = Key {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false }
        };
        assert_eq!(minibuffer.handle_key(ctrl_p), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().input, "old_command");

        // 履歴次へ
        let ctrl_n = Key {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false }
        };
        assert_eq!(minibuffer.handle_key(ctrl_n), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().input, "");
    }
}
```

### 補完機能テスト (`test_completion_system`)

```rust
#[cfg(test)]
mod completion_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_completion() {
        // テンポラリディレクトリ作成
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // テストファイル作成
        fs::write(temp_path.join("test1.txt"), "").unwrap();
        fs::write(temp_path.join("test2.txt"), "").unwrap();
        fs::write(temp_path.join("other.txt"), "").unwrap();

        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some(&format!("{}/te", temp_path.display())));

        // 補完候補を確認
        assert!(minibuffer.state().completions.len() >= 2);
        assert!(minibuffer.state().completions.iter().any(|c| c.contains("test1.txt")));
        assert!(minibuffer.state().completions.iter().any(|c| c.contains("test2.txt")));
        assert!(!minibuffer.state().completions.iter().any(|c| c.contains("other.txt")));
    }

    #[test]
    fn test_completion_limit() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("/"));

        // 50個制限の確認
        assert!(minibuffer.state().completions.len() <= 50);
    }

    #[test]
    fn test_tab_completion() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("test"));

        // 補完候補があることを想定
        minibuffer.state.completions = vec!["test.txt".to_string(), "test.md".to_string()];
        minibuffer.state.selected_completion = Some(0);

        let tab_key = Key { code: KeyCode::Tab, modifiers: KeyModifiers::default() };
        assert_eq!(minibuffer.handle_key(tab_key), MinibufferResult::Continue);
        assert_eq!(minibuffer.state().input, "test.txt");
    }
}
```

### エラーハンドリングテスト (`test_error_handling`)

```rust
#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_invalid_file_path() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("/invalid/path/that/does/not/exist"));

        // エラー表示確認のためのシミュレーション
        minibuffer.show_error("ファイルが見つかりません: /invalid/path".to_string());

        match &minibuffer.state().mode {
            MinibufferMode::ErrorDisplay { message, .. } => {
                assert!(message.contains("ファイルが見つかりません"));
            }
            _ => panic!("Expected ErrorDisplay mode"),
        }
    }

    #[test]
    fn test_error_message_expiry() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.show_error("Test error".to_string());

        // エラー表示中
        assert!(matches!(minibuffer.state().mode, MinibufferMode::ErrorDisplay { .. }));

        // 時間経過をシミュレート（実際の実装では Thread::sleep 等を使用）
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 任意のキーでエラー消去
        let key = Key::char('a');
        assert_eq!(minibuffer.handle_key(key), MinibufferResult::Continue);
        assert!(matches!(minibuffer.state().mode, MinibufferMode::Inactive));
    }

    #[test]
    fn test_cancel_operation() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("test"));

        // C-g でキャンセル
        let cancel_key = Key {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers { ctrl: true, alt: false, shift: false }
        };
        assert_eq!(minibuffer.handle_key(cancel_key), MinibufferResult::Cancel);
        assert!(matches!(minibuffer.state().mode, MinibufferMode::Inactive));
    }
}
```

## 統合テスト

### ファイル操作フローテスト (`test_file_operation_flow`)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_complete_find_file_flow() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let mut minibuffer = ModernMinibuffer::new();

        // 1. ファイル検索開始
        minibuffer.start_find_file(None);
        assert!(matches!(minibuffer.state().mode, MinibufferMode::FindFile));

        // 2. パス入力
        let path_str = test_file.to_string_lossy();
        for ch in path_str.chars() {
            minibuffer.handle_key(Key::char(ch));
        }
        assert_eq!(minibuffer.state().input, path_str);

        // 3. 実行
        let enter_key = Key { code: KeyCode::Enter, modifiers: KeyModifiers::default() };
        let result = minibuffer.handle_key(enter_key);
        assert_eq!(result, MinibufferResult::Execute(format!("find-file {}", path_str)));

        // 4. 非アクティブ化確認
        assert!(matches!(minibuffer.state().mode, MinibufferMode::Inactive));

        // 5. 履歴確認
        assert_eq!(minibuffer.state().history.len(), 1);
        assert_eq!(minibuffer.state().history.get_entry(0), Some(&path_str.to_string()));
    }

    #[test]
    fn test_completion_with_file_navigation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // テストファイル群作成
        for i in 0..10 {
            std::fs::write(temp_path.join(format!("file{:02}.txt", i)), "").unwrap();
        }

        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some(&format!("{}/file", temp_path.display())));

        // 補完候補があることを確認
        assert!(!minibuffer.state().completions.is_empty());

        // 候補ナビゲーション
        let down_key = Key { code: KeyCode::Down, modifiers: KeyModifiers::default() };
        assert_eq!(minibuffer.handle_key(down_key), MinibufferResult::Continue);

        let up_key = Key { code: KeyCode::Up, modifiers: KeyModifiers::default() };
        assert_eq!(minibuffer.handle_key(up_key), MinibufferResult::Continue);

        // Tab補完
        let tab_key = Key { code: KeyCode::Tab, modifiers: KeyModifiers::default() };
        assert_eq!(minibuffer.handle_key(tab_key), MinibufferResult::Continue);

        // 入力が更新されていることを確認
        assert!(minibuffer.state().input.contains("file"));
        assert!(minibuffer.state().input.ends_with(".txt"));
    }
}
```

## プロパティベーステスト

### 文字列操作の不変条件テスト

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_input_invariants(input_chars in prop::collection::vec(any::<char>(), 0..100)) {
            let mut minibuffer = ModernMinibuffer::new();
            minibuffer.start_find_file(None);

            // 文字を順次入力
            for ch in input_chars.iter() {
                if ch.is_control() { continue; }
                minibuffer.handle_key(Key::char(*ch));
            }

            // 不変条件：カーソル位置は有効範囲内
            let char_count = minibuffer.state().input.chars().count();
            prop_assert!(minibuffer.state().cursor_pos <= char_count);

            // 不変条件：入力文字列は有効なUTF-8
            prop_assert!(minibuffer.state().input.is_ascii() || std::str::from_utf8(minibuffer.state().input.as_bytes()).is_ok());
        }

        #[test]
        fn test_cursor_movement_invariants(
            initial_input in "[a-zA-Z0-9./]{0,50}",
            movements in prop::collection::vec(prop::sample::select(vec![
                CursorDirection::Left,
                CursorDirection::Right,
                CursorDirection::Home,
                CursorDirection::End,
            ]), 0..20)
        ) {
            let mut minibuffer = ModernMinibuffer::new();
            minibuffer.start_find_file(Some(&initial_input));

            let initial_char_count = initial_input.chars().count();

            for direction in movements {
                minibuffer.move_cursor(direction);

                // 不変条件：カーソルは常に有効範囲内
                prop_assert!(minibuffer.state().cursor_pos <= initial_char_count);
            }
        }

        #[test]
        fn test_history_invariants(
            entries in prop::collection::vec("[a-zA-Z0-9./]{1,20}", 0..50)
        ) {
            let mut history = history::SessionHistory::new();

            for entry in entries {
                history.add_entry(entry);

                // 不変条件：履歴サイズは上限を超えない
                prop_assert!(history.len() <= 100);

                // 不変条件：重複なし（最新の1つのみ残る）
                if let Some(latest) = history.get_entry(0) {
                    let count = history.iter().filter(|e| *e == latest).count();
                    prop_assert_eq!(count, 1);
                }
            }
        }
    }
}
```

## パフォーマンステスト

### 応答性能テスト

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cursor_movement_performance() {
        let mut minibuffer = ModernMinibuffer::new();
        let large_input = "a".repeat(10000);
        minibuffer.start_find_file(Some(&large_input));

        let start = Instant::now();
        minibuffer.move_cursor(CursorDirection::Home);
        minibuffer.move_cursor(CursorDirection::End);
        let duration = start.elapsed();

        // QA.mdの要件：< 1ms
        assert!(duration.as_millis() < 1, "Cursor movement took {}ms", duration.as_millis());
    }

    #[test]
    fn test_completion_performance() {
        let mut minibuffer = ModernMinibuffer::new();
        minibuffer.start_find_file(Some("/usr/"));

        let start = Instant::now();
        minibuffer.update_completions();
        let duration = start.elapsed();

        // 補完は50個制限で高速であるべき
        assert!(duration.as_millis() < 100, "Completion took {}ms", duration.as_millis());
        assert!(minibuffer.state().completions.len() <= 50);
    }
}
```

## テスト実行戦略

### 継続的インテグレーション

```bash
# 基本テスト実行
cargo test minibuffer

# プロパティベーステスト（詳細）
cargo test minibuffer -- --ignored

# パフォーマンステスト
cargo test performance_tests --release

# カバレッジ測定
cargo tarpaulin --packages altre --out Html
```

### テストデータ管理

- テンポラリディレクトリでのファイル操作テスト
- 既知のシステムパスでの補完テスト
- Unicode文字セットでの入力テスト
- 境界値（空文字列、最大長文字列）でのテスト

### 品質保証指標

- **コードカバレッジ**: 85%以上
- **ユニットテスト**: 全公開APIのカバー
- **統合テスト**: 実使用シナリオのカバー
- **パフォーマンステスト**: QA.md要件の遵守

この仕様により、ミニバッファシステムの品質と信頼性を体系的に保証する。