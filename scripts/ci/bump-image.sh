#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
sha="${1:-$(git -C "$repo_root" rev-parse HEAD)}"

printf '%s\n' "$sha" > "$repo_root/ci/image.sha"
