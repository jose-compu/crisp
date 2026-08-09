#!/usr/bin/env bash
# Point this clone at the versioned hooks in .githooks/
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push

echo "Installed git hooks (core.hooksPath=.githooks)."
echo "pre-commit and pre-push will run: cargo fmt --all -- --check"
