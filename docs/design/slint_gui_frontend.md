# Slint GUIフロントエンド設計

> **注意:** 2025-03-15 時点で Slint ベース GUI 実装は撤去されました。本書は過去の設計記録として保持しています。Tauri ベース GUI については別途設計ドキュメントを作成予定です。

## 概要
本ドキュメントでは、Slint を用いた GUI フロントエンドの構成と、共通バックエンド（CoreBackend）との連携方式を定義する。GUI モードは altre のデフォルト起動形態とし、TUI モードと同一バイナリ内で共存する。

## 目標
- TUI で提供しているミニバッファ・エディタ領域・モードラインを GUI 上で再現する
- Rust 駆動のイベントループを維持しつつ、Slint コンポーネントとデータバインディングする
- 日本語含む多言語入力・表示に対応し、将来的な UI 拡張（ウィンドウ分割、テーマ変更等）に備える

## アーキテクチャ
```
┌────────────────────────────┐
│   main.rs (CLI)            │  ← モード判定
└──────────────┬─────────────┘
               │
┌──────────────▼─────────────┐
│ CoreBackend (バックエンド) │  ← 編集状態・コマンド処理
└──────────────┬─────────────┘
               │ ViewModelSnapshot + BackendEvent
┌──────────────▼─────────────┐
│ GuiFrontend (Slint)        │
│   ├─ AppWindow (Main.slint)│  ← ルートウィンドウ
│   ├─ EditorView            │  ← テキスト表示
│   ├─ MinibufferView        │  ← コマンド/メッセージ表示
│   └─ ModeLineView          │  ← ステータス表示
└────────────────────────────┘
```

### モジュール構成（案）
```
frontend/gui/
├── mod.rs
├── app.rs              # GuiFrontend 実装（run, event loop）
├── bridge.rs           # Backendイベント変換、ViewModelSnapshot → Slint データ反映
├── components/
│   ├── main.slint      # AppWindow, レイアウト
│   ├── editor.slint    # エディタ領域
│   ├── minibuffer.slint
│   └── modeline.slint
└── theme.rs            # GUI テーマ設定
```

## コンポーネント設計

### 1. AppWindow (`main.slint`)
- レイアウト: 垂直方向に `MinibufferView` → `EditorView` → `ModeLineView` を配置。
- `EditorView` は残り領域を占有し、将来的なウィンドウ分割は `Repeater` + 動的データで対応。
- カラースキームは `theme.rs` から受け取り、`Palette` としてバインド。

### 2. EditorView (`editor.slint`)
- 表示要素:
  - 行番号（オプション: 左側サイドバー）
  - テキスト行の `Vec<ModelLine>` を `ListView` で描画
  - カーソル位置は `Rectangle` overlay または `FocusItem` で表現
  - 選択範囲は背景色変更で表示
- バインド:
  - `lines: [ModelLine]`
  - `cursor: ModelCursor`
  - `scroll: ScrollState`（トップライン、表示列オフセットなど）
- 入力処理:
  - キーイベントは `GuiFrontend` が Slint の keyboard event を受けて `FrontendEvent` に変換し、バックエンドへ送信。
  - マウスイベントは MVP では未対応。将来追加する場合は `PointerEvent` で拡張。

### 3. MinibufferView (`minibuffer.slint`)
- モード別表示:
  - `Inactive` の場合は透過バー
  - `Prompt` / `Message` / `Error` など `enum MinibufferMode`
- 補完候補:
  - `ListView` による候補表示（最大 10 行）
  - 候補選択は現状キーボードで操作（GUI でのクリック操作は将来検討）

### 4. ModeLineView (`modeline.slint`)
- 表示内容:
  - 変更フラグ、バッファ名、カーソル位置、行数、エンコーディング等
- 将来的な情報追加を想定し、`ModeLineInfo` を struct として受け取る。

## データモデル
- `src/core/backend.rs` に `RenderMetadata` / `RenderView` を追加し、バックエンドが描画用データ（ステータスライン情報 / ハイライト / ミニバッファ状態）を提供する。
- `src/frontend/gui/mod.rs` では `update_view` 関数が `TextEditor`・`MinibufferSystem` から得た状態を `EditorData` / `MinibufferData` / `ModeLineData`（`.slint` で定義）へ変換し、`AppWindow` に反映する。
- 更新方式:
  - Rust → Slint: プロパティ setter (`set_editor`, `set_minibuffer`, `set_modeline`) を利用し、Slint 側で差分描画される。
  - Slint → Rust: `EditorView` の `key_event` コールバックを通じて `slint::platform::KeyEvent` を `crossterm::event::KeyEvent` に変換し、`Backend::handle_key_event` に渡す。

## イベントハンドリング
- Rust 側が主導するイベントループを維持する。
  1. `GuiApplication::run` で `AppWindow` を生成し、`slint::Timer` により `Backend::process_minibuffer_timer` を定期実行。
  2. キーイベントは `EditorView` の `key-pressed` から `GuiApplication::setup_callbacks` に渡り、`slint_to_crossterm` で変換の上 `Backend::handle_key_event` に委譲。
  3. 入力処理・タイマー処理後は `update_view` で Slint 側のプロパティを更新し、表示差分を描画。
- キーイベント変換:
  ```rust
  fn on_key_pressed(&self, event: slint::platform::KeyEvent) {
      let frontend_event = FrontendEvent::from(event);
      let response = backend.handle_event(frontend_event);
      self.update_view(response.snapshot);
  }
  ```
- ミニバッファ入力などのテキスト編集は `Slint` のテキスト入力機構を活用し、プリエディット文字列も backend に通知する設計とする（IME 対応タスク参照）。

## テーマとスタイル
- `theme.rs` に GUI 用のカラーパレットとフォント設定を定義。
- Slint では `@globals` を用いて色・フォントを集中管理し、ダーク/ライト切り替えは将来別タスクで拡張。
- 行番号や選択範囲のスタイルは TUI 版と概ね統一しつつ、GUI ならではの余白やアンチエイリアスを活かす。

## ファイル構成とビルド
- `.slint` ファイルは `slint::include_slint!` で組み込み、`frontend/gui/components/mod.rs` でまとめて公開。
- Cargo feature `gui`（仮）で Slint 依存を有効化する。デフォルト feature で有効にし、TUI 用には `--no-default-features --features tui` を将来使う想定も可。

## 今後の課題
- ウィンドウ分割やタブ UI の設計（別タスク）
- マウス操作・ドラッグ選択の仕様策定
- パフォーマンス計測（大規模バッファの描画最適化）
- 国際化（メッセージのローカライズ）とフォント切り替え

## 参照
- ADR 0005: GUI フレームワーク選定（Slint）
- ADR 0006: GUI 統合戦略
- `docs/design/ui_backend_frontend_separation.md`
