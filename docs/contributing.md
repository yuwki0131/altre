# altre コントリビューションガイド

## 1. 開発環境のセットアップ
1. Rust 1.74 以上をインストール（`rustup` 推奨）
2. リポジトリをクローン
   ```bash
   git clone https://github.com/altre-editor/altre.git
   cd altre
   ```
3. 依存クレートの取得
   ```bash
   cargo fetch
   ```
4. 動作確認
   ```bash
   cargo test
   cargo run -- --help # raw mode が許可されていない環境では失敗する場合あり
   ```

## 2. ブランチ運用
- `main`: 常に安定版
- `feature/*`: 機能開発用
- `fix/*`: バグ修正用
- PR は `main` へ向けて作成し、GitHub Actions が通ることを確認

## 3. コーディング規約
- Rustfmt / Clippy を必ず通す
  ```bash
  cargo fmt
  cargo clippy --all-targets --all-features
  ```
- 命名規則: `snake_case` / `PascalCase` を遵守
- 公開 API（`pub`）には `///` ドキュメントコメントを付与
- モジュール間参照は `pub use` による再エクスポートで明示
- 非 ASCII 文字は既存ファイルが使用している場合のみ追加

## 4. テスト方針
- 単体テスト: 各モジュールに `#[cfg(test)]` で配置
- 結合テスト: `app/tests/`
- プロパティテスト: `proptest` を活用し、失敗時は `PROPTEST_CASE_SEED` を記録
- ベンチマーク: `cargo bench --offline`
- UI 操作の自動化は未実装のため、ミニバッファ操作は手動テストで確認

## 5. コミット & PR
- コミットメッセージは英語で簡潔に（例: `feat: add minibuffer write-file prompt`）
- PR には以下を含める
  - 変更概要
  - テスト結果
  - 関連 Issue / タスクのリンク
- ドラフト PR を活用して早期レビューを歓迎

## 6. ドキュメント
- `docs/design/` は設計仕様、`docs/architecture/` はモジュール図
- ユーザー向け文書は `manuals/`
- 変更を伴う場合は `CHANGES.md` の該当日付に追記
- 仕様に不明点がある場合は `QA.md` に質問を追加
- API ドキュメントは `scripts/generate_docs.sh` を利用して更新

## 7. Issue ライフサイクル
1. Issue 作成（テンプレートに従う）
2. `triage` ラベルで優先度分類
3. 担当者が決まったら `in-progress` ラベルを付与し、対応タスクを `tasks/proceeding/` へ移動
4. 完了後 `tasks/done/` へ移し、PR をクローズ

## 8. セキュリティとライセンス
- 未公開機能や脆弱性は Security Issue で報告
- コードは MIT / Apache-2.0 デュアルライセンス
- サードパーティ依存を追加する際はライセンス互換性を確認

## 9. サポート窓口
- Discord（非公開）: コアメンバーとの即時連絡
- GitHub Discussions: 仕様議論
- メール: security@altre.dev（脆弱性報告専用）

貢献していただきありがとうございます。質問や提案があれば遠慮なく Issue を立ててください。
