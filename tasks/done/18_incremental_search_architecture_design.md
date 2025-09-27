# インクリメンタル検索アーキテクチャ設計

## タスク概要
Emacsライクなインクリメンタル検索機能のアーキテクチャと基本設計を行う。

## 目的
- インクリメンタル検索の技術アーキテクチャ策定
- 高性能でEmacsライクなユーザー体験の実現
- 拡張可能な検索システム基盤の構築

## アーキテクチャ設計

### コアモジュール構成
```
search/
├── mod.rs              # 検索モジュール統合
├── incremental.rs      # インクリメンタル検索エンジン
├── matcher.rs          # 文字列マッチングアルゴリズム
├── highlight.rs        # 検索結果ハイライト
├── state.rs           # 検索状態管理
└── history.rs         # 検索履歴管理（将来実装）
```

### 検索状態管理システム
```rust
/// 検索方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// 検索マッチ結果
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// インクリメンタル検索状態
#[derive(Debug, Clone)]
pub struct IncrementalSearchState {
    /// 検索パターン
    pub pattern: String,

    /// 検索方向
    pub direction: SearchDirection,

    /// 現在の検索位置
    pub current_position: usize,

    /// 検索開始位置（C-gでここに戻る）
    pub start_position: usize,

    /// マッチした位置のリスト
    pub matches: Vec<SearchMatch>,

    /// 現在選択中のマッチインデックス
    pub current_match_index: Option<usize>,

    /// 検索が失敗したか
    pub failed: bool,

    /// 検索が折り返したか
    pub wrapped: bool,
}
```

### 検索エンジン設計
```rust
/// インクリメンタル検索エンジン
pub struct IncrementalSearch {
    state: IncrementalSearchState,
    matcher: Box<dyn StringMatcher>,
    case_sensitive: bool,
}

/// 文字列マッチング戦略
pub trait StringMatcher {
    fn find_matches(&self, text: &str, pattern: &str, case_sensitive: bool) -> Vec<SearchMatch>;
    fn find_next(&self, text: &str, pattern: &str, start: usize, direction: SearchDirection, case_sensitive: bool) -> Option<SearchMatch>;
}

/// 基本的なリテラル文字列マッチャー
pub struct LiteralMatcher;

/// 正規表現マッチャー（将来実装）
pub struct RegexMatcher {
    regex: regex::Regex,
}
```

### ユーザーインターフェース統合
```rust
/// 検索UI状態
pub struct SearchUI {
    /// ミニバッファとの統合
    pub minibuffer_prompt: String,

    /// 現在のステータスメッセージ
    pub status_message: Option<String>,

    /// ハイライト情報
    pub highlights: Vec<HighlightRange>,
}

/// ハイライト範囲
#[derive(Debug, Clone)]
pub struct HighlightRange {
    pub start: usize,
    pub end: usize,
    pub highlight_type: HighlightType,
}

#[derive(Debug, Clone)]
pub enum HighlightType {
    CurrentMatch,    // 現在のマッチ（強調）
    OtherMatch,     // その他のマッチ（弱い強調）
}
```

### パフォーマンス最適化設計
1. **インクリメンタルマッチング**
   - 前の検索結果を活用した差分検索
   - パターン追加時の効率的な絞り込み

2. **レンダリング最適化**
   - 画面に表示される範囲のみハイライト処理
   - 遅延ハイライトレンダリング

3. **メモリ効率化**
   - 大きなファイルでの検索結果キャッシュ管理
   - 不要な検索状態の自動クリーンアップ

### キーバインド統合
```rust
/// 検索関連のコマンド
#[derive(Debug, Clone)]
pub enum SearchCommand {
    /// インクリメンタル検索開始
    StartIncrementalSearch(SearchDirection),

    /// 検索パターンに文字追加
    AddToSearchPattern(char),

    /// 検索パターンから文字削除
    DeleteFromSearchPattern,

    /// 次/前の検索結果に移動
    MoveToNextMatch,
    MoveToPreviousMatch,

    /// 検索終了（現在位置に留まる）
    ExitSearch,

    /// 検索キャンセル（元の位置に戻る）
    CancelSearch,

    /// カーソル位置の単語を検索パターンに追加
    AddWordAtCursor,

    /// kill ringの内容を検索パターンに追加（将来実装）
    AddFromKillRing,
}
```

### エラーハンドリング
```rust
/// 検索エラーの種類
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("検索パターンが無効です: {pattern}")]
    InvalidPattern { pattern: String },

    #[error("正規表現エラー: {message}")]
    RegexError { message: String },

    #[error("検索結果が見つかりませんでした: {pattern}")]
    NotFound { pattern: String },

    #[error("検索状態が無効です")]
    InvalidState,
}
```

### テスト戦略
1. **単体テスト**
   - 各検索アルゴリズムの正確性テスト
   - エッジケース（空文字列、大きなファイル等）のテスト

2. **統合テスト**
   - キーバインドとの統合テスト
   - ミニバッファとの連携テスト

3. **パフォーマンステスト**
   - 大きなファイルでの応答性テスト
   - メモリ使用量の測定

### 実装フェーズ
1. **フェーズ1**: 基本的なリテラル文字列検索
2. **フェーズ2**: インクリメンタル検索UI統合
3. **フェーズ3**: パフォーマンス最適化
4. **フェーズ4**: 正規表現対応（将来実装）

## 依存関係
- TextEditorとの統合
- ミニバッファシステム
- キーバインドシステム
- テキストハイライトシステム

## 成果物
- 検索モジュールのスケルトン実装
- インターフェース設計仕様書
- パフォーマンス要件定義書

## 完了条件
- [ ] アーキテクチャ設計の完成
- [ ] インターフェース定義の完了
- [ ] 実装計画の策定完了
- [ ] テスト戦略の明確化

## 進捗記録
- 作成日：2025-01-28
- 状態：設計フェーズ