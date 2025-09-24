#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
APP_DIR="$REPO_ROOT/app"

cd "$APP_DIR"

cargo doc --no-deps "$@"

echo "Documentation generated at $APP_DIR/target/doc/altre/index.html"
