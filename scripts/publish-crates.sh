#!/usr/bin/env bash
# Publish Crisp workspace crates to crates.io in dependency order.
# Default: dry-run only. Pass --execute to publish for real.
#
# Package names (not directory paths): the CLI package is `crisp-lang`
# (bins `crisp` + `reveal`; lives under crates/crpc/). See docs/CRATES_IO.md.
#
# Note: dry-run for crate N only succeeds after crates 1..N-1 exist on
# crates.io (or for the first crate). Real publish walks the list in order.
# Dev-dependencies are ignored for ordering (e.g. crisp-rust-emit → crisp-lsp).
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
  echo
  echo "After publish, end users install:"
  echo "  cargo install crisp-lang --locked   # bins: crisp, reveal"
  echo "  cargo install crisp-lsp --locked"
  exit 0
fi

# Bottom-up publish order (runtime deps only; see docs/CRATES_IO.md).
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
  crisp-lang
  crisp-lsp
)

# Fail fast if the hardcoded list drifts from the workspace (Bash 3.2-safe).
python3 - "$ROOT" "${CRATES[@]}" <<'PY'
import json, subprocess, sys
root, *listed = sys.argv[1:]
meta = json.loads(
    subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
    )
)
workspace = sorted(p["name"] for p in meta["packages"])
listed_set, workspace_set = set(listed), set(workspace)
unknown = sorted(listed_set - workspace_set)
missing = sorted(workspace_set - listed_set)
if unknown or missing:
    if unknown:
        print("error: publish list has unknown package(s):", ", ".join(unknown), file=sys.stderr)
    if missing:
        print("error: workspace package(s) missing from publish list:", ", ".join(missing), file=sys.stderr)
        print("       update CRATES=() in scripts/publish-crates.sh and docs/CRATES_IO.md", file=sys.stderr)
    sys.exit(1)
print("==> publish list matches workspace (%d packages)" % len(listed))
PY

if [[ "$EXECUTE" -eq 0 ]]; then
  echo "==> dry-run only (pass --execute to publish)"
  echo "==> tip: only crisp-ast dry-runs cleanly until prior crates are on crates.io"
else
  echo "==> PUBLISHING to crates.io (${#CRATES[@]} packages, last: crisp-lang + crisp-lsp)"
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
  echo "Install check:"
  echo "  cargo install crisp-lang --locked && crisp --version"
  echo "  cargo install crisp-lsp --locked"
fi
