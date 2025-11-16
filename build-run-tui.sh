#!/usr/bin/env bash
set -euo pipefail

show_usage() {
  cat <<'USAGE'
使い方: ./build-run-tui.sh [追加引数...]

  -h, --help : このヘルプを表示します。

追加引数は `cargo run -p altre -- --tui` にそのまま渡されます。
USAGE
}

if [[ $# -gt 0 ]]; then
  case "$1" in
    -h|--help)
      show_usage
      exit 0
      ;;
  esac
fi

echo "==> Rust ワークスペースをビルド中 (release)"
cargo build --workspace --release

echo "==> TUI モードを起動中 (cargo run -p altre -- --tui)"
cargo run -p altre -- --tui "$@"
