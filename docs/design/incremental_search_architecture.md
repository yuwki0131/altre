# インクリメンタル検索アーキテクチャ設計

## 概要
本書は、altreにおけるインクリメンタル検索機能のアーキテクチャ設計を定義する。検索開始から終了までの状態遷移、テキストモデルとの連携、UI通知、性能最適化の観点を整理し、高拡張性なモジュール構成を提示する。

## 設計目標
1. **即応性**: 入力1文字あたり100ms以内に検索結果を反映
2. **拡張容易性**: 将来の正規表現検索・alisp連携を想定した抽象化
3. **再利用性**: 置換機能やナビゲーションと共有可能な検索API
4. **堅牢性**: 状態管理とエラーハンドリングの明確化による一貫した挙動

## コンポーネント構成
```
search/
├── mod.rs
├── incremental.rs      // 検索エンジン
├── state.rs            // 状態管理
├── matcher.rs          // マッチャー抽象
├── controller.rs       // コマンド制御
└── highlight.rs        // ハイライト管理
```

### 主要構造体
```rust
/// 検索方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// マッチ結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub range: std::ops::Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// インクリメンタル検索状態
#[derive(Debug, Clone)]
pub struct IncrementalSearchState {
    pub active: bool,
    pub pattern: String,
    pub direction: SearchDirection,
    pub start_position: usize,
    pub current_position: usize,
    pub matches: Vec<SearchMatch>,
    pub current_index: Option<usize>,
    pub wrapped: bool,
    pub failed: bool,
}

impl IncrementalSearchState {
    pub fn new() -> Self { /* デフォルト初期化 */ }
}

/// 検索エンジン（MVP版）
pub struct IncrementalSearchEngine<M: StringMatcher> {
    state: IncrementalSearchState,
    matcher: M,
    options: SearchOptions,
}
```

### マッチャー抽象
```rust
pub trait StringMatcher {
    fn find_first(
        &self,
        text: &str,
        pattern: &str,
        start: usize,
        direction: SearchDirection,
        options: &SearchOptions,
    ) -> Option<SearchMatch>;

    fn collect_visible(
        &self,
        text: &str,
        pattern: &str,
        viewport: &TextViewport,
        options: &SearchOptions,
    ) -> Vec<SearchMatch>;
}

/// MVPでは単純リテラルマッチャーを提供
pub struct LiteralMatcher;
```

## 処理フロー

### 1. 検索開始
1. `SearchController::start(direction)` が呼び出される
2. `IncrementalSearchState` を初期化し、`start_position` に現在のポイントを記録
3. `SearchUIAdapter` を通じてミニバッファにプロンプトを表示
4. 初回は空パターンのためマッチ探索を行わず、入力待機状態に遷移

### 2. パターン更新
1. ユーザー入力（文字追加/削除）が `SearchController` に到達
2. `IncrementalSearchEngine::update_pattern` が呼ばれ、`pattern` を更新
3. `StringMatcher::find_first` により新しいマッチポイントを取得
4. 成功時: `current_position` と `current_index` を更新し、ハイライトイベントを発火
5. 失敗時: `failed = true` としてUIにエラー表示を依頼
6. 折り返しが必要な場合は `SearchWrapDetector` が補助し、1度だけ通知

### 3. 移動操作
- `C-s`/`C-r` 操作は `SearchController::move_next/prev` にルーティング
- 現在の `matches` を参照し、必要に応じて追加探索
- 画面上に表示するマッチ集合は `collect_visible` で都度再構築

### 4. 終了・キャンセル
- `Enter`: `state.active = false` にし、最後の `current_position` を確定
- `C-g`: `start_position` にカーソルを戻し、`state` を初期化
- いずれも `SearchUIAdapter::clear` を呼んでハイライトを解除

## UI統合
- `SearchUIAdapter` がミニバッファ表示と本文ハイライト制御の橋渡しを担う。
- ミニバッファには `SearchPromptState`（モード、パターン、件数、折り返しフラグ）を渡す。
- 本文ハイライトは `HighlightRequest` として `ui/highlight.rs` に通知し、レンダリングフレームで適用する。

## 状態遷移図
```
┌────────────┐   文字入力    ┌──────────────┐
│ Idle       │────────────▶│ AwaitingInput │
└────────────┘               └──────┬───────┘
       ▲    C-s/C-r開始             │ 成功/失敗
       │                            ▼
       │                         ┌───────────────┐
 C-g   │                         │ Matching      │
       │◀────────────────────────│ (Active)      │
       │   Enter確定/Cancel      └──────┬────────┘
       │                                │ move next/prev
       │                                ▼
       │                         ┌───────────────┐
       └─────────────────────────│ Navigating    │
                                 └───────────────┘
```

## 性能最適化ポイント
- **差分マッチング**: パターンに1文字追加された際、既存マッチ集合からのフィルタリングでO(k)（k:既存マッチ数）に抑制。
- **部分ハイライト**: 表示中のウィンドウ範囲外はマッチ計算を遅延し、スクロール時に再配信。
- **LRUキャッシュ**: 最近使用した検索語とマッチ位置をLRUで保持し、同一クエリの再検索を高速化。

## エラーハンドリング
| エラー種別 | 発生条件 | 対応 |
| --- | --- | --- |
| `SearchError::EmptyPattern` | 文字削除で空文字列になった場合 | マッチリストを初期化し成功扱い |
| `SearchError::NotFound` | マッチが見つからない | 状態は維持、UIに失敗表示 |
| `SearchError::InvalidUtf8Boundary` | ギャップバッファの境界計算で失敗 | `AltreError`に昇格し操作を中断 |

## テスト方針
- 単体テスト: `incremental.rs` の状態遷移、`matcher.rs` のマッチング精度
- 統合テスト: TUIイベント→検索→ハイライトのエンドツーエンド確認
- プロパティテスト: ランダム文字列上での差分マッチングの整合性検証
- ベンチマーク: 1MBテキスト、10文字パターンでのレイテンシ測定

## 将来拡張
- `RegexMatcher` 実装による正規表現検索
- 検索履歴のリングバッファ化とミニバッファ履歴連携
- 検索結果のストリーミング通知（大規模ファイル対応）

