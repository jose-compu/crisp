#!/usr/bin/env bash
# Before git commit/push from the agent: require rustfmt clean (matches CI).
set -euo pipefail

input="$(cat)"
command="$(printf '%s' "$input" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("command") or "")' 2>/dev/null || true)"

if [[ ! "$command" =~ git[[:space:]]+(push|commit) ]]; then
  printf '%s\n' '{"permission":"allow"}'
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' '{"permission":"allow"}'
  exit 0
fi

# Apply formatting first so the subsequent commit/push can include it if needed.
cargo fmt --all >/dev/null 2>&1 || true

if ! cargo fmt --all -- --check >/dev/null 2>&1; then
  printf '%s\n' '{
    "permission": "deny",
    "user_message": "rustfmt check failed. Run: cargo fmt --all",
    "agent_message": "Blocked git commit/push: cargo fmt --all -- --check failed. Run cargo fmt --all, stage the changes, then retry."
  }'
  exit 0
fi

printf '%s\n' '{"permission":"allow"}'
exit 0
