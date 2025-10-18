# IME検証手法調査

## 概要
GUI/TUI 共通で日本語 IME を含む多言語入力を保証するための検証手法を調査し、今後の実装および手動確認フローの指針を定める。

## 調査対象
- Linux (Wayland/Hyprland, X11)
- Windows 10/11
- macOS Sonoma 以降
- TUI（Crossterm）と Slint GUI の両モード

## 自動化候補
| 手法 | 対応プラットフォーム | メリット | デメリット |
|------|----------------------|----------|------------|
| `ibus` / `fcitx5` スクリプト駆動 | Linux | IME をプログラムから制御可能 | 変換候補選択が難しく、環境依存が大きい |
| `slint-testing`（将来予定） | GUI | Slint が提供する UI テストツールでキー入力を再現 | 2025-09 現在、IME 入力イベントのサポートは限定的 |
| `AutoHotkey` / `PowerShell` | Windows | キーシーケンス自動化が容易 | 文字変換結果の取得が難しく、環境設定が必要 |
| `AppleScript` + `osascript` | macOS | 公式 API でキーイベント送出可能 | IME 状態取得が困難、セキュリティ許可が必要 |

現状、IME 変換結果の検証を完全自動化する手段は成熟しておらず、手動確認とログ取得を組み合わせるアプローチが現実的と判断する。

## 推奨方針
1. **Automated Smoke（半自動）**
   - 文字入力（ASCII）については自動テストでカバー（既存ユニットテスト）。
   - IME のプリエディット挙動は、Slint/TUI から受け取るイベントログを検証用ログに保存し、手動確認時の参考にする。
   - GUI/TUI それぞれで「IME 有効時に Backspace/Cancel が正しく動作する」程度のスモークテストを用意。
2. **Manual Checklist**
   - OS × ディスプレイサーバ毎に以下を確認:
     - ひらがな入力 → 漢字変換 → 確定
     - 半角/全角切替
     - 英数モードとの切り替え
     - 途中で `Esc`/`Ctrl+G` キャンセル（プリエディット破棄）
     - `Ctrl` 系ショートカットとの競合（`Ctrl+Space`, `Ctrl+/` 等）
   - チェック結果を `docs/design/tests/ime_checklist.md`（後続タスクで作成）に記録。
3. **ログ収集**
   - `ALTRE_DEBUG=1` で IME 関連イベントを標準エラーへ出力。
   - GUI/TUI ともにプリエディットテキスト、確定文字列、キャンセルイベントをログ化。
4. **将来の自動化**
   - Slint 側の自動テスト拡張（issue 追跡）をウォッチし、IME 入力を扱える API が公開された場合に再評価。
   - Linux では `dbus-send` を使った IME 切替や `ydotool` を用いたキー入力自動化を検証予定。

## 手動検証フロー（ドラフト）
1. 指定 OS のチェックリストに沿って IME 操作を実施。
2. 観測した挙動（成功/失敗）を記録し、必要に応じてスクリーンキャプチャを保存。
3. 問題が発生した場合は再現手順とログを `tasks/todo/bugs` に登録。
4. リリース前チェックとして GUI/TUI 双方で最低 1 回実施。

## 参考資料
- Slint ドキュメント: Text Input & IME Handling
- Crossterm issue tracker: IME 対応状況
- Hyprland Wiki: IME サポートと設定

## 関連タスク
- `tasks/todo/design/gui_regression_test_strategy.md`
- `tasks/todo/future/gui_regression_tests_implementation.md`
- `tasks/todo/future/gui_documentation_update.md`
