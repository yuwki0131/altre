# タスク管理方法

## 基本方針
* タスクはtasksディレクトリ配下のmarkdownファイルで一元管理
* 1タスク1markdownファイルとする
* 1タスク中にさらに細分化タスクが発生した場合は複数のmarkdownファイルに分解して管理
* 抽象的なタスクも1タスクとして管理、具体化するフェーズでtodoにタスクを追加してく形とする

## タスクライフサイクル
1. **作業開始時**: `tasks/todo/` から `tasks/proceeding/` に移動
2. **作業中**: `tasks/proceeding/` 内で管理（進捗更新等）
3. **作業完了時**: `tasks/proceeding/` から `tasks/done/` に移動
4. **進捗記録**: README.mdの「最新の進捗状況」セクションに完了記録を追加

## 重要な運用ルール
- **必須**: 作業開始時は必ず該当タスクを `proceeding/` に移動する
- **必須**: 作業完了時は必ず該当タスクを `done/` に移動する
- **推奨**: 長期タスクは定期的に進捗をタスクファイル内に記録する

## タスク管理ディレクトリのディレクトリ構成

* tasks: task管理用ディレクトリのルートディレクトリ
  * tasks/todo/mvp: 未実施タスク(MVP実装)
  * tasks/todo/alisp: alisp実装タスク
  * tasks/todo/bugs: 不具合対応タスク
  * tasks/todo/future: 将来的に必要になるタスク。現時点では未実施でOK
  * tasks/todo/design: 設計タスク
  * tasks/proceeding: 進行中タスク
  * tasks/done: 完了タスク
