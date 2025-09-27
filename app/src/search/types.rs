//! 検索関連の共通型

/// 検索方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    /// 前方検索
    Forward,
    /// 後方検索
    Backward,
}

impl SearchDirection {
    /// ラベルを取得
    pub fn label(self) -> &'static str {
        match self {
            SearchDirection::Forward => "I-search",
            SearchDirection::Backward => "I-search backward",
        }
    }
}

/// 検索状態の表示ステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStatus {
    /// 検索中（マッチあり）
    Active,
    /// マッチが見つからない
    NotFound,
    /// 折り返し検索が発生
    Wrapped,
}

/// 1件の検索マッチ情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// マッチ開始位置（文字インデックス）
    pub start: usize,
    /// マッチ終了位置（文字インデックス、排他的）
    pub end: usize,
    /// マッチ開始行
    pub line: usize,
    /// マッチ開始列
    pub column: usize,
}

impl SearchMatch {
    /// マッチ長（文字数）
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// マッチが空か判定
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// UI描画用のハイライト情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHighlight {
    /// 行番号
    pub line: usize,
    /// 行内開始位置（文字単位）
    pub start_column: usize,
    /// 行内終了位置（文字単位、排他的）
    pub end_column: usize,
    /// 現在のマッチであるか
    pub is_current: bool,
}

/// ミニバッファに表示する検索UI状態
#[derive(Debug, Clone)]
pub struct SearchUiState {
    /// ラベル（I-searchなど）
    pub prompt_label: String,
    /// 現在の検索パターン
    pub pattern: String,
    /// ステータス
    pub status: SearchStatus,
    /// 現在のマッチ番号（1-based）
    pub current_match: Option<usize>,
    /// 総マッチ数
    pub total_matches: usize,
    /// 折り返しが発生したか
    pub wrapped: bool,
    /// メッセージ（失敗や通知）
    pub message: Option<String>,
    /// 検索方向
    pub direction: SearchDirection,
}

impl SearchUiState {
    /// エラー状態かを判定
    pub fn is_error(&self) -> bool {
        matches!(self.status, SearchStatus::NotFound)
    }
}
