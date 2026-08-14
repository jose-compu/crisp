#!/usr/bin/env bash
# Publish Crisp workspace crates to crates.io in dependency order.
# Default: dry-run only. Pass --execute to publish for real.
#
# Package names (not directory paths): the CLI package is `crisp-lang`
# (bins `crisp` + `reveal`; lives under crates/crpc/). See docs/CRATES_IO.md.
#
# Skips crate@version pairs already on crates.io. Sleeps between uploads to
# reduce 429 rate-limit hits (crates.io limits new-crate bursts).
# Override pause with PUBLISH_SLEEP=<seconds> (default: 30).
#
# Dev-dependencies are ignored for ordering (e.g. crisp-rust-emit → crisp-lsp).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

EXECUTE=0
PUBLISH_SLEEP="${PUBLISH_SLEEP:-30}"

if [[ "${1:-}" == "--execute" ]]; then
  EXECUTE=1
elif [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "Usage: $0 [--execute]"
  echo "  (default) cargo publish --dry-run for each crate"
  echo "  --execute  cargo publish for each crate (needs cargo login)"
  echo
  echo "Already-published crate@version pairs are skipped."
  echo "Sleep between successful uploads: ${PUBLISH_SLEEP}s (env PUBLISH_SLEEP)."
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

VERSION="$(
  cargo metadata --no-deps --format-version 1 \
    | python3 -c '
import json, sys
pkgs = {p["name"]: p["version"] for p in json.load(sys.stdin)["packages"]}
print(pkgs["crisp-ast"])
'
)"

# Returns 0 if crates.io already has name@version.
crate_version_exists() {
  local name="$1" ver="$2" code
  code="$(
    curl -sS -o /dev/null -w '%{http_code}' \
      -A 'crisp-publish-crates (https://github.com/jose-compu/crisp)' \
      "https://crates.io/api/v1/crates/${name}/${ver}"
  )"
  case "$code" in
    200) return 0 ;;
    404) return 1 ;;
    *)
      echo "    warning: crates.io lookup HTTP $code for ${name}@${ver}; assuming not published" >&2
      return 1
      ;;
  esac
}

if [[ "$EXECUTE" -eq 0 ]]; then
  echo "==> dry-run only (pass --execute to publish)"
  echo "==> workspace version ${VERSION}; skip if already on crates.io"
else
  echo "==> PUBLISHING to crates.io (${#CRATES[@]} packages @ ${VERSION})"
  echo "==> sleep ${PUBLISH_SLEEP}s between successful uploads (PUBLISH_SLEEP)"
fi

FAILED=0
SKIPPED=0
PUBLISHED=0

for name in "${CRATES[@]}"; do
  echo
  echo "==> ${name} @ ${VERSION}"

  if crate_version_exists "$name" "$VERSION"; then
    echo "    skip: already on crates.io (${name}@${VERSION})"
    SKIPPED=$((SKIPPED + 1))
    sleep 1
    continue
  fi

  if [[ "$EXECUTE" -eq 0 ]]; then
    if ! cargo publish -p "$name" --dry-run --allow-dirty; then
      echo "    (dry-run failed — often because prior crates are not on crates.io yet)"
      FAILED=1
    fi
  else
    set +e
    out="$(cargo publish -p "$name" 2>&1)"
    status=$?
    set -e
    printf '%s\n' "$out"

    if [[ $status -eq 0 ]]; then
      PUBLISHED=$((PUBLISHED + 1))
      echo "    published ${name}@${VERSION}"
      echo "    sleeping ${PUBLISH_SLEEP}s (rate-limit cushion)…"
      sleep "$PUBLISH_SLEEP"
    elif printf '%s\n' "$out" | grep -qi 'already exists on crates.io'; then
      echo "    skip: cargo reports already on crates.io (${name}@${VERSION})"
      SKIPPED=$((SKIPPED + 1))
      sleep 1
    elif printf '%s\n' "$out" | grep -qi '429 Too Many Requests\|rate.limit\|too many new crates'; then
      echo
      echo "error: crates.io rate limit hit while publishing ${name}@${VERSION}" >&2
      echo "       Wait for the time in the cargo message above, then re-run:" >&2
      echo "         ./scripts/publish-crates.sh --execute" >&2
      echo "       Already-published crates will be skipped." >&2
      exit 1
    else
      echo "error: cargo publish failed for ${name}@${VERSION} (exit ${status})" >&2
      exit "$status"
    fi
  fi
done

echo
echo "==> summary: skipped=${SKIPPED} published=${PUBLISHED} version=${VERSION}"
if [[ "$EXECUTE" -eq 0 ]]; then
  if [[ "$FAILED" -ne 0 ]]; then
    echo "Some dry-runs failed (expected until dependencies are published)."
    echo "Re-run with --execute when ready; already-published crates are skipped."
  else
    echo "Dry-run finished. Re-run with --execute when ready."
  fi
else
  if [[ "$PUBLISHED" -eq 0 && "$SKIPPED" -eq "${#CRATES[@]}" ]]; then
    echo "Nothing to do — all ${#CRATES[@]} packages already at ${VERSION} on crates.io."
  else
    echo "Publish pass finished."
  fi
  echo "Install check:"
  echo "  cargo install crisp-lang --locked && crisp --version"
  echo "  cargo install crisp-lsp --locked"
fi
