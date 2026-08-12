#!/usr/bin/env bash
# Package editors/vscode-crisp as a VSIX (#57).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXT="$ROOT/editors/vscode-crisp"
cd "$EXT"
if ! command -v npx >/dev/null 2>&1; then
  echo "npx required (Node.js). Install Node, then re-run." >&2
  exit 1
fi
npx --yes @vscode/vsce package --no-dependencies
echo "VSIX written under $EXT — install via: Extensions: Install from VSIX…"
