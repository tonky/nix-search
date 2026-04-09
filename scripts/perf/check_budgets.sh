#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

out_dir="${1:-tmp/bench/perf-size-ci}"
mkdir -p "$out_dir"

PERF_MODE="${PERF_MODE:-quick}"
if [[ "$PERF_MODE" != "quick" && "$PERF_MODE" != "full" ]]; then
  echo "invalid PERF_MODE: $PERF_MODE (expected quick|full)" >&2
  exit 2
fi

RUN_TRUNK_BUILD="${RUN_TRUNK_BUILD:-auto}"
if [[ "$RUN_TRUNK_BUILD" == "auto" ]]; then
  if [[ "$PERF_MODE" == "full" ]]; then
    RUN_TRUNK_BUILD="1"
  else
    RUN_TRUNK_BUILD="0"
  fi
fi

RUN_LATENCY_PROBE="${RUN_LATENCY_PROBE:-auto}"
if [[ "$RUN_LATENCY_PROBE" == "auto" ]]; then
  if [[ "$PERF_MODE" == "full" ]]; then
    RUN_LATENCY_PROBE="1"
  else
    RUN_LATENCY_PROBE="0"
  fi
fi

TOTAL_STEPS=6

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

# Size ceilings (bytes). Adjust as improvements land.
WASM_MAX_RAW="${WASM_MAX_RAW:-4500000}"
WASM_MAX_BROTLI="${WASM_MAX_BROTLI:-800000}"
JS_MAX_RAW="${JS_MAX_RAW:-100000}"
JS_MAX_BROTLI="${JS_MAX_BROTLI:-12000}"
DATA_MAX_BROTLI="${DATA_MAX_BROTLI:-2600000}"
BROTLI_QUALITY="${BROTLI_QUALITY:-9}"
if [[ "$PERF_MODE" == "full" ]]; then
  DATA_BROTLI_QUALITY="${DATA_BROTLI_QUALITY:-9}"
else
  DATA_BROTLI_QUALITY="${DATA_BROTLI_QUALITY:-4}"
fi

# Runtime thresholds (milliseconds).
SEARCH_P95_WARN_MS="${SEARCH_P95_WARN_MS:-40}"
if [[ "$PERF_MODE" == "full" ]]; then
  SEARCH_P95_FAIL_MS="${SEARCH_P95_FAIL_MS:-120}"
else
  SEARCH_P95_FAIL_MS="${SEARCH_P95_FAIL_MS:-0}"
fi

failures=0
warnings=0

manifest="crates/nix-search-web/static/data/manifest.json"
progress 1 "Checking prepared artifact inputs"
if [[ ! -f "$manifest" ]]; then
  echo "missing manifest: $manifest" >&2
  exit 2
fi

artifact_rel="$(jq -r '.artifact' "$manifest")"
artifact="crates/nix-search-web/static/data/${artifact_rel}"
if [[ ! -f "$artifact" ]]; then
  echo "missing prepared artifact: $artifact" >&2
  exit 2
fi

build_log="$out_dir/01-trunk-build.log"
progress 2 "Building release web bundle for budget measurement"
if [[ "$RUN_TRUNK_BUILD" == "1" ]]; then
  run_with_heartbeat \
    "trunk build --release" \
    "$build_log" \
    bash -lc 'cd crates/nix-search-web && trunk build --release --dist ../../tmp/trunk-dist-budget'
else
  echo "skipped trunk build (PERF_MODE=$PERF_MODE, RUN_TRUNK_BUILD=$RUN_TRUNK_BUILD)" > "$build_log"
  echo "  -> trunk build skipped; reusing existing dist artifacts if available"
fi

dist_dir="tmp/trunk-dist-budget"
if ! compgen -G "${dist_dir}/nix-search-web-*_bg.wasm" >/dev/null; then
  dist_dir="tmp/trunk-dist-web"
fi
if ! compgen -G "${dist_dir}/nix-search-web-*_bg.wasm" >/dev/null; then
  echo "missing web dist artifacts. Run just web-build once, or set RUN_TRUNK_BUILD=1." >&2
  exit 2
fi

wasm_file="$(ls "${dist_dir}"/nix-search-web-*_bg.wasm | head -n 1)"
js_file="$(ls "${dist_dir}"/nix-search-web-*.js | head -n 1)"
data_file="$artifact"

measure_file() {
  local path="$1"
  local label="$2"
  local quality="$3"
  local raw
  local br
  local started
  local elapsed

  started="$(date +%s)"
  echo "  -> [$label] measuring $(basename "$path") (brotli q=${quality})" >&2
  raw="$(wc -c < "$path" | tr -d ' ')"
  local br_tmp
  br_tmp="tmp/perf-brotli-${label}-$$.br"
  run_with_heartbeat \
    "brotli [$label]" \
    "$out_dir/measure-${label}.log" \
    bash -lc "brotli -q '$quality' -c '$path' > '$br_tmp'"
  br="$(wc -c < "$br_tmp" | tr -d ' ')"
  rm -f "$br_tmp"
  elapsed="$(( $(date +%s) - started ))"
  echo "  <- [$label] done in ${elapsed}s" >&2
  printf "%s\t%s\t%s\n" "$path" "$raw" "$br"
}

size_report="$out_dir/02-size-report.tsv"
progress 3 "Measuring wasm/js/data raw and brotli sizes"
{
  echo -e "file\traw_bytes\tbrotli_bytes"
  measure_file "$wasm_file" "wasm" "$BROTLI_QUALITY"
  measure_file "$js_file" "js" "$BROTLI_QUALITY"
  measure_file "$data_file" "data" "$DATA_BROTLI_QUALITY"
} > "$size_report"

size_rows="$(awk -F'\t' 'NF==3 && $1 ~ /\/nix-search-web-.*_bg\.wasm$|\/nix-search-web-.*\.js$|\/packages-.*\.json$/ {print $0}' "$size_report")"
wasm_raw="$(printf '%s\n' "$size_rows" | awk -F'\t' '$1 ~ /_bg\.wasm$/ {print $2; exit}')"
wasm_br="$(printf '%s\n' "$size_rows" | awk -F'\t' '$1 ~ /_bg\.wasm$/ {print $3; exit}')"
js_raw="$(printf '%s\n' "$size_rows" | awk -F'\t' '$1 ~ /\/nix-search-web-.*\.js$/ {print $2; exit}')"
js_br="$(printf '%s\n' "$size_rows" | awk -F'\t' '$1 ~ /\/nix-search-web-.*\.js$/ {print $3; exit}')"
data_br="$(printf '%s\n' "$size_rows" | awk -F'\t' '$1 ~ /\/packages-.*\.json$/ {print $3; exit}')"

if [[ -z "$wasm_raw" || -z "$wasm_br" || -z "$js_raw" || -z "$js_br" || -z "$data_br" ]]; then
  echo "failed to parse size report rows from $size_report" >&2
  cat "$size_report" >&2 || true
  exit 2
fi

progress 4 "Evaluating size budget thresholds"
if (( wasm_raw > WASM_MAX_RAW )); then
  echo "FAIL wasm raw size ${wasm_raw} > ${WASM_MAX_RAW}" | tee -a "$out_dir/03-budget-check.txt"
  failures=$((failures + 1))
fi
if (( wasm_br > WASM_MAX_BROTLI )); then
  echo "FAIL wasm brotli size ${wasm_br} > ${WASM_MAX_BROTLI}" | tee -a "$out_dir/03-budget-check.txt"
  failures=$((failures + 1))
fi
if (( js_raw > JS_MAX_RAW )); then
  echo "FAIL js raw size ${js_raw} > ${JS_MAX_RAW}" | tee -a "$out_dir/03-budget-check.txt"
  failures=$((failures + 1))
fi
if (( js_br > JS_MAX_BROTLI )); then
  echo "FAIL js brotli size ${js_br} > ${JS_MAX_BROTLI}" | tee -a "$out_dir/03-budget-check.txt"
  failures=$((failures + 1))
fi
if [[ "$PERF_MODE" == "full" ]]; then
  if (( data_br > DATA_MAX_BROTLI )); then
    echo "FAIL data brotli size ${data_br} > ${DATA_MAX_BROTLI}" | tee -a "$out_dir/03-budget-check.txt"
    failures=$((failures + 1))
  fi
else
  echo "WARN data brotli budget is informational in quick mode (measured with q=${DATA_BROTLI_QUALITY})" | tee -a "$out_dir/03-budget-check.txt"
  warnings=$((warnings + 1))
fi

probe_log="$out_dir/04-latency-probe.log"
progress 5 "Running latency probe (warning-level threshold)"
search_p95=""
if [[ "$RUN_LATENCY_PROBE" == "1" ]]; then
  run_with_heartbeat \
    "latency probe" \
    "$probe_log" \
    cargo run -p nix-search-web --bin latency_probe --release -- --artifact "$artifact" --iterations 40
  search_p95="$(awk -F'=' '/^search_p95_ms=/{print $2}' "$probe_log" | tr -d ' ')"
  if [[ -n "$search_p95" ]]; then
    p95_int="${search_p95%.*}"
    if (( SEARCH_P95_FAIL_MS > 0 )) && (( p95_int > SEARCH_P95_FAIL_MS )); then
      echo "FAIL search_p95_ms ${search_p95} > ${SEARCH_P95_FAIL_MS}" | tee -a "$out_dir/03-budget-check.txt"
      failures=$((failures + 1))
    fi
    if (( p95_int > SEARCH_P95_WARN_MS )); then
      echo "WARN search_p95_ms ${search_p95} > ${SEARCH_P95_WARN_MS}" | tee -a "$out_dir/03-budget-check.txt"
      warnings=$((warnings + 1))
    fi
  fi
else
  echo "skipped latency probe (PERF_MODE=$PERF_MODE, RUN_LATENCY_PROBE=$RUN_LATENCY_PROBE)" > "$probe_log"
  echo "  -> latency probe skipped"
fi

summary="$out_dir/summary.txt"
progress 6 "Writing summary and final status"
{
  echo "size budgets: failures=$failures warnings=$warnings"
  echo "mode=$PERF_MODE run_trunk_build=$RUN_TRUNK_BUILD run_latency_probe=$RUN_LATENCY_PROBE"
  echo "wasm_raw=$wasm_raw wasm_brotli=$wasm_br"
  echo "js_raw=$js_raw js_brotli=$js_br"
  echo "data_brotli=$data_br data_brotli_quality=$DATA_BROTLI_QUALITY"
  if [[ -n "${search_p95:-}" ]]; then
    echo "search_p95_ms=$search_p95"
  fi
  echo "search_p95_warn_ms=$SEARCH_P95_WARN_MS search_p95_fail_ms=$SEARCH_P95_FAIL_MS"
} > "$summary"

cat "$summary"

if (( failures > 0 )); then
  exit 1
fi
