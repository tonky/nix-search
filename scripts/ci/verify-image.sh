#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

if [[ ! -x /usr/local/bin/ci ]]; then
  echo "missing baked ci binary at /usr/local/bin/ci" >&2
  exit 1
fi

cargo build -p ci --release --locked

baked_hash="$(/usr/local/bin/ci --version | sha256sum | awk '{print $1}')"
rebuilt_hash="$(target/release/ci --version | sha256sum | awk '{print $1}')"

if [[ "$baked_hash" != "$rebuilt_hash" ]]; then
  echo "ci binary drift detected" >&2
  echo "baked:   $baked_hash" >&2
  echo "rebuilt: $rebuilt_hash" >&2
  exit 1
fi
