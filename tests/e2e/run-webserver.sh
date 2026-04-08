#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

# Prepare deterministic local data for the web app.
flox activate -- just prep-web

# Build static web assets once (no watch loop).
cd crates/nix-search-web
flox activate -- trunk build

# Serve compiled dist for Playwright tests.
cd dist
exec flox activate -- python3 -m http.server 4173 --bind 127.0.0.1
