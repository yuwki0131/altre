# altre ロゴ管理メモ

| ファイル | 用途 |
| --- | --- |
| `altre-logo-original.png` | 提供された原稿データ（無加工） |
| `altre-logo.png` | 透明余白をトリミングした整形済みロゴ（汎用） |
| `altre-logo-readme.png` | README 表示向けに幅 400px へリサイズしたバージョン |

加工コマンド例（ImageMagick 7 系）
```bash
magick altre-logo-original.png -trim +repage altre-logo.png
magick altre-logo.png -resize 400x altre-logo-readme.png
```

必要に応じて他サイズを追加する場合は、このディレクトリにまとめて管理してください。
