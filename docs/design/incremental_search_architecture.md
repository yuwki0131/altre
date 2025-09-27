# インクリメンタル検索アーキテクチャ設計

## 概要
本書は、altreにおけるインクリメンタル検索機能のアーキテクチャ設計を定義する。最新の実装では `app/src/search/` 以下に検索モジュールが配置され、`SearchController` がエディタとUI（ミニバッファ）を仲介する役割を担う。

## 設計目標
1. **即応性**: 入力1文字あたり100ms以内に検索結果を反映
2. **拡張容易性**: 将来の正規表現検索・alisp連携を想定した抽象化
3. **再利用性**: 置換機能やナビゲーションと共有可能な検索API
4. **堅牢性**: 状態管理とエラーハンドリングの明確化による一貫した挙動

## モジュール構成（実装版）
```
search/
├── matcher.rs          # リテラルマッチャー（ケースフォールディング対応）
├── mod.rs              # SearchController と公開API
├── state.rs            # 検索状態 (SearchState)
└── types.rs            # SearchDirection / SearchMatch / SearchHighlight などの型定義
```

> 初期草案に記載された `incremental.rs` / `controller.rs` / `highlight.rs` は実装統合により `mod.rs` と `text_area.rs` 側に吸収された。

## SearchController の責務
- `TextEditor` から文字列とカーソル位置を取得し、`SearchState` に反映する。
- `LiteralMatcher` を用いてマッチ集合を再計算し、`SearchHighlight` を生成。
- `SearchUiState` を構築してミニバッファ表示や状態メッセージを制御。
- `C-s` / `C-r` / `C-g` / `Enter` といった検索専用キーを処理する。

## 主なデータ構造
```rust
pub struct SearchState {
    pub active: bool,
    pub pattern: String,
    pub direction: SearchDirection,
    pub matches: Vec<SearchMatch>,
    pub current_index: Option<usize>,
    pub wrapped: bool,
    pub failed: bool,
    pub start_cursor: Option<CursorPosition>,
    pub start_char_index: usize,
}
```
- `active`: 検索モードが継続中かどうか。
- `start_cursor`: `C-g` キャンセル時に復帰するカーソル位置。
- `wrapped`: 折り返し検索が発生したか。
- `failed`: 現在のパターンで一致がない場合に true。

## 処理フロー
1. **開始 (`start`)**
   - 状態を初期化し、直前の検索語があれば引き継ぐ。
   - マッチ済みであればカーソル近傍から最初の一致を選択。
2. **文字追加 (`input_char`)**
   - パターン末尾に文字を追加し、マッチ集合を再計算。
   - ケースフォールディング（小文字のみ → 大文字小文字無視）を適用。
3. **文字削除 (`delete_char`)**
   - パターンを1文字短くし、空文字の場合はハイライトとUIを初期化。
4. **移動 (`repeat_forward` / `repeat_backward`)**
   - 現在の `matches` から次/前のマッチを選択。端に到達したら折り返し。
5. **終了 (`accept`) / キャンセル (`cancel`)**
   - `Enter`: 状態を終了し、最後に移動した位置を保持。
   - `C-g`: `start_cursor` に戻して状態を完全リセット。

## ハイライト生成
- `SearchHighlight` には行番号と列範囲、および現在マッチ（太字表示）かどうかを格納。
- `TextArea::prepare_lines` で `Span` に変換し、現在マッチは背景シアン、その他は `Color::Rgb(0, 80, 80)` を使用。

## ミニバッファ連携
- `SearchUiState` に次の情報を格納:
  - `prompt_label` (`I-search` / `I-search backward`)
  - `pattern`
  - `status` (`Active`/`Wrapped`/`NotFound`)
  - `current_match` / `total_matches`
  - `message`: 失敗や折り返しをユーザーに通知
- `App::handle_key_event` で検索中は専用ハンドラにディスパッチ。
- ミニバッファが情報メッセージを表示している場合でも `Ctrl+s` で即座に検索へ移行するよう `MinibufferSystem::is_message_displayed` を追加（2025-09-27 修正）。

## 非機能要件
- **性能**: 1MB程度のテキストで100ms以内の応答。
- **安全性**: UTF-8 文字境界を保持し、結合文字も正しく扱う。
- **テスト**: `cargo test -- --test-threads=1` で 240 件のテストが成功することを確認。

## 今後の拡張
- 正規表現検索 (`RegexMatcher`) の追加とグループハイライト。
- 検索語履歴のリングバッファ化（`M-p`/`M-n` で履歴移動）。
- 大規模ファイルのための段階的サンプリングやバックグラウンド検索。
- alisp コマンドからの検索API公開。

