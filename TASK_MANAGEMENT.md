# タスク管理方法

* タスクはtasksディレクトリ配下のmarkdownファイルで一元管理
* 1タスク1markdownファイルとする
* 1タスク中にさらに細分化タスクが発生した場合は複数のmarkdownファイルに分解して管理
* 抽象的なタスクも1タスクとして管理、具体化するフェーズでtodoにタスクを追加してく形とする

## タスク管理ディレクトリのディレクトリ構成

* tasks: task管理用ディレクトリのルートディレクトリ
  * tasks/todo/mvp: 未実施タスク(MVP実装)
  * tasks/todo/alisp: alisp実装タスク
  * tasks/todo/future: 将来的に必要になるタスク。現時点では未実施でOK
  * tasks/todo/design: 設計タスク
  * tasks/proceeding: 進行中タスク
  * tasks/done: 完了タスク
