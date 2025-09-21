# プロジェクト構造セットアップ

## タスク概要
MVPで必要なRustプロジェクトの基本構造とモジュール配置を整備する。

## 目的
- 設計に基づいたモジュール構造の実装
- 依存関係の整理と最適化
- 開発効率とコードの保守性を向上
- テスト環境の基盤整備

## 実装対象
1. **ディレクトリ構造の整備**
   ```
   app/src/
   ├── main.rs
   ├── lib.rs
   ├── buffer/          # テキストバッファ関連
   │   ├── mod.rs
   │   └── gap_buffer.rs
   ├── input/           # 入力処理関連
   │   ├── mod.rs
   │   └── keybinding.rs
   ├── ui/              # UI関連
   │   ├── mod.rs
   │   ├── layout.rs
   │   └── minibuffer.rs
   ├── file/            # ファイル操作関連
   │   ├── mod.rs
   │   └── operations.rs
   ├── error.rs         # エラーハンドリング
   └── config.rs        # 設定管理（将来用）
   ```

2. **Cargo.tomlの最適化**
   - 依存関係の整理
   - feature flags の設定
   - 開発依存関係の追加

3. **基本モジュールのスケルトン作成**
   - 各モジュールの pub interface 定義
   - エラー型の実装
   - 基本的なテストフレームワーク

4. **開発環境の設定**
   - rustfmt.toml
   - clippy.toml
   - VSCode settings（参考）

## 成果物
- 完全に整備された `app/src/` ディレクトリ
- 最適化された `Cargo.toml`
- 基本テストの実行確認
- 開発環境設定ファイル

## 前提条件
- 01_architecture_design.md の完了
- 既存のCargo.tomlと依存関係の確認

## 完了条件
- [ ] ディレクトリ構造の作成
- [ ] 各モジュールのスケルトン実装
- [ ] `cargo build` の成功
- [ ] `cargo test` の成功
- [ ] `cargo clippy` の警告ゼロ

## 見積もり
**期間**: 1-2日
**優先度**: 最高（他の実装の前提）

## 関連タスク
- 01_architecture_design.md（前提）
- 他のすべてのMVP実装タスク（後続）

## 技術的考慮事項
- モジュール間の循環依存の回避
- testディレクトリの活用
- 将来のalisp統合を考慮したモジュール境界
- パフォーマンス計測用のbenchmarkディレクトリ準備