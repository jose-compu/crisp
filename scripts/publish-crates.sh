#!/usr/bin/env bash
# Publish Crisp workspace crates to crates.io in dependency order.
# Default: dry-run only. Pass --execute to publish for real.
#
# Package names (not directory paths): the CLI package is `crisp-lang`
# (bins `crisp` + `reveal`; lives under crates/crpc/). See docs/CRATES_IO.md.
#
# Skips crate@version already on crates.io. On 429, parses crates.io's
# "try again after <date>", waits, and retries (does not exit).
#
# crates.io new-crate limits: burst of 5, then 1 every 10 minutes.
# New versions of existing crates: burst of 30, then 1 per minute.
# Defaults: NEW_CRATE_SLEEP=620, VERSION_SLEEP=65 (seconds). Override via env.
#
# Dev-dependencies are ignored for ordering (e.g. crisp-rust-emit → crisp-lsp).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

EXECUTE=0
# Prefer explicit PUBLISH_SLEEP if set; else per-kind defaults below.
NEW_CRATE_SLEEP="${NEW_CRATE_SLEEP:-620}"
VERSION_SLEEP="${VERSION_SLEEP:-65}"
PUBLISH_SLEEP="${PUBLISH_SLEEP:-}"

if [[ "${1:-}" == "--execute" ]]; then
  EXECUTE=1
elif [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "Usage: $0 [--execute]"
  echo "  (default) cargo publish --dry-run for each crate"
  echo "  --execute  cargo publish for each crate (needs cargo login)"
  echo
  echo "Already-published crate@version pairs are skipped."
  echo "On 429 rate limits: wait until crates.io retry-after, then retry."
  echo "Pace after success:"
  echo "  new crate name:  ${NEW_CRATE_SLEEP}s  (NEW_CRATE_SLEEP; crates.io = 10 min)"
  echo "  new version:     ${VERSION_SLEEP}s   (VERSION_SLEEP; crates.io = 1 min)"
  echo "  override both:   PUBLISH_SLEEP=<seconds>"
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

http_get_code() {
  local url="$1"
  curl -sS -o /dev/null -w '%{http_code}' \
    -A 'crisp-publish-crates (https://github.com/jose-compu/crisp)' \
    "$url"
}

# Returns 0 if crates.io already has name@version.
crate_version_exists() {
  local name="$1" ver="$2" code
  code="$(http_get_code "https://crates.io/api/v1/crates/${name}/${ver}")"
  case "$code" in
    200) return 0 ;;
    404) return 1 ;;
    *)
      echo "    warning: crates.io lookup HTTP $code for ${name}@${ver}; assuming not published" >&2
      return 1
      ;;
  esac
}

# Returns 0 if the crate name exists on crates.io (any version).
crate_name_exists() {
  local name="$1" code
  code="$(http_get_code "https://crates.io/api/v1/crates/${name}")"
  case "$code" in
    200) return 0 ;;
    404) return 1 ;;
    *)
      echo "    warning: crates.io lookup HTTP $code for ${name}; treating as new crate" >&2
      return 1
      ;;
  esac
}

# Sleep until crates.io "try again after <HTTP-date>" (plus buffer), or fallback seconds.
wait_for_rate_limit() {
  local cargo_out="$1" fallback_secs="${2:-620}"
  local secs
  secs="$(
    printf '%s\n' "$cargo_out" | python3 -c '
import sys, re
from datetime import datetime, timezone
text = sys.stdin.read()
# e.g. Please try again after Fri, 14 Aug 2026 11:59:11 GMT
m = re.search(r"try again after\s+([A-Za-z]{3},\s+\d{1,2}\s+[A-Za-z]{3}\s+\d{4}\s+\d{2}:\d{2}:\d{2}\s+GMT)", text, re.I)
if not m:
    print("")
    sys.exit(0)
raw = m.group(1)
try:
    when = datetime.strptime(raw, "%a, %d %b %Y %H:%M:%S GMT").replace(tzinfo=timezone.utc)
except ValueError:
    print("")
    sys.exit(0)
now = datetime.now(timezone.utc)
wait = (when - now).total_seconds() + 15  # small cushion past the stated time
print(int(max(wait, 1)))
'
  )"
  if [[ -z "$secs" ]]; then
    secs="$fallback_secs"
    echo "    rate limit: could not parse retry-after; waiting ${secs}s fallback…"
  else
    echo "    rate limit: waiting ${secs}s until crates.io retry-after (+15s cushion)…"
  fi
  # Progress ticks so the console does not look hung.
  local remaining="$secs" chunk
  while [[ "$remaining" -gt 0 ]]; do
    if [[ "$remaining" -gt 60 ]]; then
      chunk=60
    else
      chunk="$remaining"
    fi
    sleep "$chunk"
    remaining=$((remaining - chunk))
    if [[ "$remaining" -gt 0 ]]; then
      echo "    … ${remaining}s left before retry"
    fi
  done
}

sleep_after_success() {
  local is_new_crate="$1"
  local secs
  if [[ -n "$PUBLISH_SLEEP" ]]; then
    secs="$PUBLISH_SLEEP"
  elif [[ "$is_new_crate" -eq 1 ]]; then
    secs="$NEW_CRATE_SLEEP"
  else
    secs="$VERSION_SLEEP"
  fi
  echo "    sleeping ${secs}s before next upload (crates.io pace)…"
  sleep "$secs"
}

publish_with_retry() {
  local name="$1"
  local is_new_crate="$2"
  local attempt=1
  local out status

  while true; do
    echo "    publish attempt ${attempt}…"
    set +e
    out="$(cargo publish -p "$name" 2>&1)"
    status=$?
    set -e
    printf '%s\n' "$out"

    if [[ $status -eq 0 ]]; then
      PUBLISHED=$((PUBLISHED + 1))
      echo "    published ${name}@${VERSION}"
      sleep_after_success "$is_new_crate"
      return 0
    fi

    if printf '%s\n' "$out" | grep -qi 'already exists on crates.io'; then
      echo "    skip: cargo reports already on crates.io (${name}@${VERSION})"
      SKIPPED=$((SKIPPED + 1))
      sleep 1
      return 0
    fi

    if printf '%s\n' "$out" | grep -qi '429 Too Many Requests\|too many new crates\|rate.limit'; then
      echo "    rate limited on ${name}@${VERSION} — will wait and retry (not exiting)"
      wait_for_rate_limit "$out" "$NEW_CRATE_SLEEP"
      attempt=$((attempt + 1))
      continue
    fi

    echo "error: cargo publish failed for ${name}@${VERSION} (exit ${status})" >&2
    return "$status"
  done
}

if [[ "$EXECUTE" -eq 0 ]]; then
  echo "==> dry-run only (pass --execute to publish)"
  echo "==> workspace version ${VERSION}; skip if already on crates.io"
else
  echo "==> PUBLISHING to crates.io (${#CRATES[@]} packages @ ${VERSION})"
  echo "==> new-crate pace ${NEW_CRATE_SLEEP}s · version pace ${VERSION_SLEEP}s"
  echo "==> on 429: wait for crates.io retry-after, then retry automatically"
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

  IS_NEW=1
  if crate_name_exists "$name"; then
    IS_NEW=0
    echo "    note: crate name exists — this is a new version (1/min pace after burst)"
  else
    echo "    note: brand-new crate name (1 per 10 min after 5-crate burst)"
  fi

  if [[ "$EXECUTE" -eq 0 ]]; then
    if ! cargo publish -p "$name" --dry-run --allow-dirty; then
      echo "    (dry-run failed — often because prior crates are not on crates.io yet)"
      FAILED=1
    fi
  else
    publish_with_retry "$name" "$IS_NEW"
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
