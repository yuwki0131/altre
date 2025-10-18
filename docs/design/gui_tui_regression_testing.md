# GUI/TUI回帰テスト戦略

## 概要
バックエンドとフロントエンドを分離し、GUI（Slint）と TUI（ratatui）を同一バイナリで提供するにあたり、機能退行を検出するためのテスト戦略を定義する。

## 方針
1. **バックエンド中心の単体テスト強化**  
   - バッファ操作、検索、ミニバッファ状態遷移などを CoreBackend の API 単位で検証する。
2. **フロントエンド固有の統合テスト**  
   - TUI と GUI でそれぞれ最小限の起動・入力シナリオを自動化する。
3. **手動検証の体系化**  
   - IME・描画品質・パフォーマンスなど自動化が難しい領域をチェックリスト化する。
4. **CI への段階的統合**  
   - 当面はローカル/手動実行が中心。必要な依存を CI に導入でき次第、自動テストを拡張する。

## テスト階層
| レベル | 対象 | 目的 | 実施形態 |
|--------|------|------|----------|
| Unit   | CoreBackend、buffer、input、search 等 | 個別機能の正当性 | `cargo test --lib --features core` |
| Integration | 共有バックエンド + フロントエンド経由 | コマンドフローの動作確認 | TUI: `cargo test --test tui_smoke` / GUI: `cargo test --test gui_smoke` |
| Scenario | 主要ユースケース（ファイル編集、検索置換） | 一連の操作の回帰検知 | スクリプト or `expect` ベース |
| Manual | IME、描画品質、パフォーマンス | 非自動領域のチェック | チェックリストに従い目視確認 |

## バックエンド単体テスト
- `CoreBackend` に対し、`apply_event` → `get_view_model` の結果を検証するテストを追加。
- 既存の buffer/minibuffer/search テストを移植し、共有モジュールの責務を確認。
- 状態遷移テーブルをベースにしたプロパティテストは将来的に `proptest` で実施。

## TUI フロントエンド統合テスト
- `cargo test --test tui_smoke` で Crossterm backend をモック化し、主要なキーシーケンスを送出してバックエンドのレスポンスを検証。
- 端末依存部分は `crossterm::event::poll` をスタブ化し、フレーム生成回数・ViewModel 反映をアサート。
- Raw mode に依存するテストは `cfg(test)` で無効化、もしくは `TERM=dumb` 下で実行可能な形に限定。

## GUI フロントエンド統合テスト
- Slint のヘッドレスレンダリング機能を活用し、ウィンドウを表示せず描画ツリーの状態を確認する。
- `cargo test --test gui_smoke` で GUI 起動 → 1 操作 → ViewModel 反映を確認。
- 依存不足で実行できない環境ではテストを `ignore` し、CI 導入時に feature で制御。

## シナリオテスト
- `scripts/tests/` に CLI ベースのシナリオテストを配置し、`altre --tui` を対話コマンドツール（例: `expect`）で操作する。
- GUI シナリオは将来的に Wavefront（Slint 提供の UI テスト）や画像差分を検討するが、現段階では優先度低。

## 手動検証チェックリスト
- **IME**: 日本語入力（かな漢字変換）、半角/全角切替、英数入力、生入力キャンセル
- **描画**: 全角文字幅、タブ表示、複数ウィンドウ、検索ハイライト
- **パフォーマンス**: 1MB ファイルのスクロール、検索速度
- **エラー表示**: ファイル保存失敗、検索未ヒット、ミニバッファメッセージ消去
- チェックリストは `docs/manuals/manuals/` ではなく `docs/design/tests/gui_tui_checklist.md`（新規）で管理予定（後続タスク）。

## CI への組み込み
- ステップ1: バックエンド単体テストを既存 CI に統合（既に `cargo test` で実行）
- ステップ2: TUI/GUI スモークテストは `--features smoke-tests` で opt-in 実行
- ステップ3: Slint 依存インストール完了後、GUI テストを CI に追加

## ログ・デバッグ支援
- `ALTRE_DEBUG=1` でバックエンドとフロントエンドのイベントログを出力し、テスト失敗時の診断を容易化する。
- テスト時はステップバイステップでログ収集するオプション（例: `--log-file`）の導入を検討。

## 参照タスク
- `tasks/todo/design/gui_regression_test_strategy.md`
- `tasks/todo/design/ime_verification_strategy_research.md`
- `tasks/todo/future/gui_regression_tests_implementation.md`
