#!/usr/bin/env bash
set -euo pipefail

show_usage() {
  cat <<'USAGE'
使い方: ./build-run.sh [gui|tui] [追加引数...]

  gui (既定) : GUI モードをビルドし実行します。フロントエンドの npm 依存取得とビルドを含みます。
  tui        : TUI モードのみをビルドし実行します。
  -h, --help : このヘルプを表示します。

追加引数は `cargo run` にそのまま渡されます。
USAGE
}

MODE="gui"

if [[ $# -gt 0 ]]; then
  case "$1" in
    gui|--gui)
      MODE="gui"
      shift
      ;;
    tui|--tui)
      MODE="tui"
      shift
      ;;
    -h|--help)
      show_usage
      exit 0
      ;;
    *)
      echo "不明なモード指定です: $1" >&2
      echo >&2
      show_usage >&2
      exit 1
      ;;
  esac
fi

echo "==> Rust ワークスペースをビルド中 (release)"
cargo build --workspace --release

if [[ "${MODE}" == "gui" ]]; then
  echo "==> フロントエンド依存を取得中 (npm install --prefix frontend/react)"
  npm install --prefix frontend/react

  echo "==> フロントエンドをビルド中 (npm run build --prefix frontend/react)"
  npm run build --prefix frontend/react

  echo "==> GUI モードを起動中 (cargo run -p altre -- --gui)"
  cargo run -p altre -- --gui "$@"
else
  echo "==> TUI モードを起動中 (cargo run -p altre -- --tui)"
  cargo run -p altre -- --tui "$@"
fi
