#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO_ROOT"

cargo doc --no-deps "$@"

echo "Documentation generated at $REPO_ROOT/target/doc/altre/index.html"
