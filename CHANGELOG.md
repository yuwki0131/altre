# altre Changelog

## [0.1.0] - 2025-09-24
### 追加
- Emacs ライクな TUI エディタ MVP を実装
- ギャップバッファベースの `TextEditor` とナビゲーション API
- ミニバッファによる `find-file` / `save-buffer` / `write-file` フロー
- `CommandProcessor` によるキーバインド連携とファイル操作の統合
- `AdvancedRenderer` によるレイアウト描画と可変ミニバッファ
- ユーザー/開発者向けドキュメント、トラブルシューティング、FAQ の整備
- `scripts/generate_docs.sh` による API ドキュメント生成フロー

### 修正
- ミニバッファの保存プロンプト導線（未保存バッファでの `C-x C-s`）
- 設計資料 (`docs/design/*`) を実装と同期

### 既知の制限
- 単一バッファのみ対応
- Undo/Redo 未実装
- GUI 版は将来実装予定

---
過去の変更履歴は `CHANGES.md` を参照してください。今後のバージョンではセマンティックバージョニングに従います。
