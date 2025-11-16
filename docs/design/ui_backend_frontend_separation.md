# UIバックエンド/フロントエンド分離設計

## 概要
本ドキュメントでは、現行の ratatui ベース TUI 実装からバックエンド（ドメイン層）とフロントエンド（表示層）を明確に分離し、将来的に Slint GUI を追加しても共通ロジックを再利用できる構成を定義する。

## 背景と目的
- `App` 構造体がターミナル制御（Crossterm）と TUI 描画（ratatui）に強く結合しており、GUI 追加時にコードの再利用が難しい。
- ADR 0006 にて GUI と TUI を並行提供する方針を採用したため、共通バックエンドとフロントエンド固有ロジックを切り分ける必要がある。
- バックエンドを共有化することで、テキスト編集・ファイル操作・検索などの主要ロジックを単一の API として維持し、TUI/GUI それぞれの描画・入力処理を独立実装できるようにする。

## 現状整理
- `src/app.rs` がアプリケーション状態とイベントループ、端末制御、描画までを一括して扱っている。
- `src/ui/` 配下は ratatui 前提の描画構造体群で構成され、他フロントエンドからは再利用しづらい。
- 入力処理（Crossterm イベントループ）も `App::event_loop` 内部で直接扱っており、抽象化されていない。

## 目標構成
```
src/
├── core/                # フロントエンド非依存のバックエンド
│   ├── app/             # 編集状態・バッファ管理・コマンド実行
│   ├── document/        # TextEditor, Buffer 等（既存 buffer/ editor を再配置）
│   ├── io/              # ファイル操作
│   ├── input/           # コマンド分派・キー処理（フロント共通部分）
│   └── search/ ...      # 既存 search 等を移動/再エクスポート
├── frontend/
│   ├── tui/             # ratatui + crossterm 実装
│   │   ├── input/       # Crossterm イベント取得
│   │   ├── render/      # 既存 ui/ モジュールを移植
│   │   └── app.rs       # TUI 専用エントリーポイント
│   └── gui/             # Slint （後続で実装）
│       ├── components/
│       └── app.rs
└── main.rs (CLI 起動)   # モード判定 → backend 初期化 → frontend 起動
```

※ 実際のディレクトリ名はタスク実装時に微調整可能。

## 分離方針
1. **バックエンド API 層**  
   - `CoreBackend`（仮称）を導入し、バッファ管理、キーバインド、検索、ミニバッファ状態などを統合的に管理する構造体を定義する。  
   - 既存 `App` のうち、端末制御・描画・イベントポーリングを除いたロジックを順次移植する。
   - 公開メソッド例:
     ```rust
     pub struct CoreBackend {
         pub fn new(config: BackendConfig) -> Result<Self>;
         pub fn handle_command(&mut self, action: BackendAction) -> BackendResponse;
         pub fn get_view_model(&self) -> ViewModelSnapshot;
         pub fn tick(&mut self) -> BackendTick;
     }
     ```
   - `ViewModelSnapshot` はフロントエンドが描画に使用するデータをまとめた読み取り専用構造体。

2. **フロントエンド抽象インターフェース**  
   - `Frontend` トレイトを想定し、`run(&mut self, backend: CoreBackend) -> Result<()>` のような共通エントリーポイントを定義する。  
   - TUI/GUI 各フロントエンドはこのトレイトを実装し、入力イベント → BackendAction 変換 → BackendResponse のサイクルを担当する。

3. **TUI フロントエンドの責務**  
   - Crossterm イベントループでキー入力を取得し、バックエンドに送信。  
   - ratatui を用いて `ViewModelSnapshot` を描画。  
   - 端末初期化（raw mode, alternate screen）と後始末を担当。

4. **バックエンドとフロントエンド間のデータバインディング**  
   - レンダリングに必要な情報（バッファ文字列、カーソル位置、マーク範囲、ステータス情報、メッセージ等）を `ViewModelSnapshot` に集約し、コピーオンライトや参照カウントを活用して過度なコピーを避ける。  
   - 入力イベントは enum `FrontendEvent` として抽象化し、TUI/GUI が固有イベントを変換したのち `CoreBackend` へ渡す。

## データフロー（TUI モード）
```
Crossterm Event → FrontendEvent → CoreBackend::handle_event()
    ↓                                  ↓
 ratatui Render ← ViewModelSnapshot ←  CoreBackend::get_view_model()
```

## マイグレーションステップ
1. **設計ドキュメント確定（本ドキュメント）**
2. **コアモジュール整理**  
   - `App` から端末制御部分を切り出し、バックエンド専用構造体（仮称 `BackendState`）を導入。
3. **TUI モジュール再配置**  
   - `src/ui/` の各ファイルを `frontend/tui/render/` へ移動し、`ratatui` 依存が他レイヤへ漏れないよう整理。
4. **インターフェース実装**  
   - `FrontendEvent`, `BackendAction`, `ViewModelSnapshot` を定義し、既存コマンド処理と接続。
5. **既存 API 互換対応**  
   - `lib.rs` で公開する構造体を更新。TUI 起動は `frontend::tui::AppRunner::run()` のような形式に変更。
6. **テスト更新**  
   - バックエンド単体テスト追加、TUI フロントエンドの smoke test（環境制約に合わせた最小検証）を用意。

## リスクと緩和策
- **リファクタリング範囲が大きい**  
  - 段階的に `App` を薄くし、バックエンド API を育てる。テスト補強と併せて進める。
- **描画用データのコピーコスト**  
  - `ViewModelSnapshot` では `Arc` やスライス参照を活用し、大容量文字列のコピーを避ける設計とする。
- **機能退行の懸念**  
  - 設計フェーズで回帰テスト戦略（別タスク）を確立し、リグレッション対策を講じる。

## 影響範囲
- `src/app.rs`, `src/ui/` 以外にも、`input`・`minibuffer`・`editor` 等のモジュールでバックエンド API に合わせた軽微なインターフェース変更が発生する可能性がある。
- ビルド機構への直接的な変更はないが、後続で GUI/TUI モード切替実装時に `main.rs` の構成が変化する。

## 今後のアクション
- 設計をもとにタスク `tui_backend_frontend_separation_implementation.md` を進める。
- GUI フロントエンド設計（Slint）と同期し、共有インターフェースを確定させる。
- テスト戦略・依存調査タスクと連携し、リファクタリング後の検証計画を具体化する。

## 実装メモ（2025-09-28）
- バックエンドは `src/core/backend.rs` として切り出し、描画や端末制御を排除して共有ロジックに集中させた。
- TUI フロントエンドは `src/frontend/tui/mod.rs` に配置し、Crossterm/Ratatui 依存とイベントループを担当。
- 描画のための `RenderMetadata` / `RenderView` をバックエンドから提供し、レンダラーとの橋渡しを明示化。
