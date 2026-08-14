#!/usr/bin/env bash
# Publish Crisp workspace crates to crates.io in dependency order.
# Default: dry-run only. Pass --execute to publish for real.
#
# Note: dry-run for crate N only succeeds after crates 1..N-1 exist on
# crates.io (or for the first crate). Real publish walks the list in order.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

EXECUTE=0
if [[ "${1:-}" == "--execute" ]]; then
  EXECUTE=1
elif [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "Usage: $0 [--execute]"
  echo "  (default) cargo publish --dry-run for each crate"
  echo "  --execute  cargo publish for each crate (needs cargo login)"
  echo
  echo "Dry-run of later crates fails until earlier ones are on crates.io;"
  echo "that is expected. Use --execute for the real ordered publish."
  exit 0
fi

# Bottom-up publish order (see docs/CRATES_IO.md).
CRATES=(
  crisp-ast
  crisp-lexer
  crisp-manifest
  crisp-diagnostics
  crisp-parser
  crisp-resolve
  crisp-typeck
  crisp-ownership
  crisp-errors
  crisp-regions
  crisp-cir
  crisp-rust-emit
  crisp-reveal
  crisp-crpc
  crisp-lsp
)

if [[ "$EXECUTE" -eq 0 ]]; then
  echo "==> dry-run only (pass --execute to publish)"
  echo "==> tip: only the first crate dry-runs cleanly until others are published"
else
  echo "==> PUBLISHING to crates.io"
fi

FAILED=0
for name in "${CRATES[@]}"; do
  echo
  echo "==> $name"
  if [[ "$EXECUTE" -eq 0 ]]; then
    if ! cargo publish -p "$name" --dry-run --allow-dirty; then
      echo "    (dry-run failed — often because prior crates are not on crates.io yet)"
      FAILED=1
    fi
  else
    cargo publish -p "$name"
    # Registry index lag: brief pause between uploads.
    sleep 5
  fi
done

echo
if [[ "$EXECUTE" -eq 0 ]]; then
  if [[ "$FAILED" -ne 0 ]]; then
    echo "Some dry-runs failed (expected until dependencies are published)."
    echo "crisp-ast dry-run should succeed; re-run with --execute when ready."
  else
    echo "Dry-run finished. Re-run with --execute when ready."
  fi
else
  echo "Publish finished."
fi
