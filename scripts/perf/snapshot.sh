#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

ts="$(date -u +%Y%m%dT%H%M%SZ)"
out_dir="${1:-tmp/bench/perf-size-${ts}}"
mkdir -p "$out_dir"

TOTAL_STEPS=9

progress() {
  local step="$1"
  local message="$2"
  local width=24
  local filled=$((step * width / TOTAL_STEPS))
  local empty=$((width - filled))
  local percent=$((step * 100 / TOTAL_STEPS))
  local fill_bar
  local empty_bar
  printf -v fill_bar '%*s' "$filled" ''
  printf -v empty_bar '%*s' "$empty" ''
  fill_bar=${fill_bar// /#}
  empty_bar=${empty_bar// /-}
  printf '[%02d/%02d] [%s%s] %3d%% %s\n' "$step" "$TOTAL_STEPS" "$fill_bar" "$empty_bar" "$percent" "$message"
}

run_with_heartbeat() {
  local label="$1"
  local log_file="$2"
  shift 2

  local started
  started="$(date +%s)"
  echo "  -> ${label}" >&2

  "$@" > "$log_file" 2>&1 &
  local cmd_pid=$!

  while kill -0 "$cmd_pid" 2>/dev/null; do
    local elapsed
    elapsed="$(( $(date +%s) - started ))"
    echo "     ${label}: ${elapsed}s elapsed" >&2
    sleep 5
  done

  wait "$cmd_pid"
  local code=$?
  local total
  total="$(( $(date +%s) - started ))"
  if [[ $code -ne 0 ]]; then
    echo "  <- ${label} failed after ${total}s (showing tail of ${log_file})" >&2
    tail -n 40 "$log_file" >&2 || true
    return $code
  fi

  echo "  <- ${label} done in ${total}s" >&2
}

target_dir="${CARGO_TARGET_DIR:-}"
if [[ -z "$target_dir" ]]; then
  progress 1 "Resolving Cargo target directory"
  target_dir="$(cargo metadata --format-version 1 --no-deps | jq -r '.target_directory')"
else
  progress 1 "Using CARGO_TARGET_DIR override"
fi
debug_bin="$target_dir/debug/nix-search"
release_bin="$target_dir/release/nix-search"

progress 2 "Collecting environment snapshot"
{
  echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  rustc -Vv
  cargo -V
  uname -a
} > "$out_dir/00-env.txt"

# Build both profiles once so runtime-only timings are less noisy.
progress 3 "Building debug binary"
run_with_heartbeat "cargo build (debug)" "$out_dir/01-build-debug.log" cargo build --bin nix-search
progress 4 "Building release binary"
run_with_heartbeat "cargo build (release)" "$out_dir/02-build-release.log" cargo build --release --bin nix-search

if [[ ! -x "$debug_bin" ]]; then
  echo "missing debug binary: $debug_bin" >&2
  exit 2
fi
if [[ ! -x "$release_bin" ]]; then
  echo "missing release binary: $release_bin" >&2
  exit 2
fi

# Compare runtime-only prep-web speed using separate outputs.
progress 5 "Timing prep-web debug and release runs"
run_with_heartbeat \
  "prep-web debug" \
  "$out_dir/11-prep-web-debug.log" \
  bash -lc "/usr/bin/time -p '$debug_bin' prep-web --output tmp/perf-snapshot-prep-debug"
run_with_heartbeat \
  "prep-web release" \
  "$out_dir/12-prep-web-release.log" \
  bash -lc "/usr/bin/time -p '$release_bin' prep-web --output tmp/perf-snapshot-prep-release"

# Compare runtime-only cache update speed using fresh local cache dirs.
progress 6 "Timing cache update debug and release runs"
rm -rf tmp/perf-snapshot-cache-debug tmp/perf-snapshot-cache-release
run_with_heartbeat \
  "cache update debug" \
  "$out_dir/13-cache-update-debug.log" \
  bash -lc "/usr/bin/time -p '$debug_bin' --cache-dir tmp/perf-snapshot-cache-debug cache update"
run_with_heartbeat \
  "cache update release" \
  "$out_dir/14-cache-update-release.log" \
  bash -lc "/usr/bin/time -p '$release_bin' --cache-dir tmp/perf-snapshot-cache-release cache update"

# Build a release web bundle if trunk is available.
if command -v trunk >/dev/null 2>&1; then
  progress 7 "Building web release bundle with trunk"
  run_with_heartbeat \
    "trunk build --release" \
    "$out_dir/20-trunk-build.log" \
    bash -lc 'cd crates/nix-search-web && trunk build --release --dist ../../tmp/trunk-dist-web-perf'
  echo "trunk_build=ok" > "$out_dir/20-web-build-status.txt"
else
  progress 7 "Skipping trunk build (not installed)"
  echo "trunk_build=skipped (trunk not installed)" > "$out_dir/20-web-build-status.txt"
fi

if ! compgen -G "tmp/trunk-dist-web-perf/nix-search-web-*.wasm" >/dev/null \
  && ! compgen -G "tmp/trunk-dist-web/nix-search-web-*.wasm" >/dev/null; then
  echo "no wasm artifact found in tmp/trunk-dist-web-perf or tmp/trunk-dist-web" >&2
  exit 2
fi

# Size report over existing dist artifact (from this run or prior run).
progress 8 "Measuring static artifact sizes"
size_report="$out_dir/21-size-report.txt"
: > "$size_report"
for f in tmp/trunk-dist-web-perf/nix-search-web-*.wasm tmp/trunk-dist-web-perf/nix-search-web-*.js tmp/trunk-dist-web-perf/data/packages-*.json tmp/trunk-dist-web/nix-search-web-*.wasm tmp/trunk-dist-web/nix-search-web-*.js tmp/trunk-dist-web/data/packages-*.json; do
  [[ -f "$f" ]] || continue
  raw="$(wc -c < "$f" | tr -d ' ')"
  gzip_size="$(gzip -9 -c "$f" | wc -c | tr -d ' ')"
  if command -v brotli >/dev/null 2>&1; then
    br_size="$(brotli -q 11 -c "$f" | wc -c | tr -d ' ')"
  else
    br_size="n/a"
  fi
  printf "%s\traw=%s\tgzip=%s\tbrotli=%s\n" "$f" "$raw" "$gzip_size" "$br_size" >> "$size_report"
done

# Record native binary sizes.
progress 9 "Writing native binary size report"
{
  ls -lh "$debug_bin" "$release_bin"
} > "$out_dir/22-native-binary-size.txt" 2>&1

summary_file="$out_dir/summary.txt"
{
  echo "perf-size snapshot: $out_dir"
  for f in "$out_dir"/11-prep-web-debug.log "$out_dir"/12-prep-web-release.log "$out_dir"/13-cache-update-debug.log "$out_dir"/14-cache-update-release.log; do
    if [[ -f "$f" ]]; then
      real="$(awk '/^real / {print $2}' "$f")"
      printf "%s real=%s\n" "$(basename "$f")" "${real:-n/a}"
    fi
  done
  echo "size report: $size_report"
} > "$summary_file"

cat "$summary_file"
