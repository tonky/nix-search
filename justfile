# Common development workflows for nix-search.

set positional-arguments

default:
    @just --list

# Format all Rust code.
fmt:
    cargo fmt --all

# Run clippy for all targets/features.
clippy:
    cargo clippy --workspace --all-targets --all-features

# Run the primary test suite.
test:
    cargo test --workspace

# Fast native + wasm checks used during staged implementation.
check:
    cargo check --workspace
    cargo check -p nix-search-web --target wasm32-unknown-unknown
    cargo check -p nix-search-web --bin latency_probe

# Build prepared web data into a local output directory.
# Reuses existing prepared artifact by default for a faster local loop.
# Pass `force=1` to regenerate from upstream data.
prep-web output="tmp/pages-data" force="0":
    @set -e; \
    out_dir="{{output}}"; \
    out_dir="${out_dir#output=}"; \
    out_dir="${out_dir#out=}"; \
    out_dir="${out_dir#out_dir=}"; \
    manifest="$out_dir/manifest.json"; \
    if [[ "{{force}}" != "1" ]] && [[ -f "$manifest" ]]; then \
        artifact="$out_dir/$(jq -r '.artifact' "$manifest")"; \
        compressed=$(jq -r '.compressed_artifact // empty' "$manifest"); \
        compressed_path="$out_dir/$compressed"; \
        has_compressed=0; \
        if [[ -n "$compressed" ]] && [[ -f "$compressed_path" ]]; then \
            has_compressed=1; \
        fi; \
        if [[ -f "$artifact" ]] && [[ "$has_compressed" == "1" ]]; then \
            echo "prep-web: using cached artifact $artifact"; \
        else \
            echo "prep-web: cached manifest/artifacts incomplete, regenerating"; \
            cargo run -- prep-web --output "$out_dir"; \
        fi; \
    else \
        cargo run -- prep-web --output "$out_dir"; \
    fi; \
    just sync-web-data "$out_dir"

# Build prepared web data with release profile for faster heavy-data transforms.
prep-web-fast output="tmp/pages-data" force="0":
    @set -e; \
    out_dir="{{output}}"; \
    out_dir="${out_dir#output=}"; \
    out_dir="${out_dir#out=}"; \
    out_dir="${out_dir#out_dir=}"; \
    manifest="$out_dir/manifest.json"; \
    if [[ "{{force}}" != "1" ]] && [[ -f "$manifest" ]]; then \
        artifact="$out_dir/$(jq -r '.artifact' "$manifest")"; \
        compressed=$(jq -r '.compressed_artifact // empty' "$manifest"); \
        compressed_path="$out_dir/$compressed"; \
        has_compressed=0; \
        if [[ -n "$compressed" ]] && [[ -f "$compressed_path" ]]; then \
            has_compressed=1; \
        fi; \
        if [[ -f "$artifact" ]] && [[ "$has_compressed" == "1" ]]; then \
            echo "prep-web-fast: using cached artifact $artifact"; \
        else \
            echo "prep-web-fast: cached manifest/artifacts incomplete, regenerating"; \
            cargo run --release -- prep-web --output "$out_dir"; \
        fi; \
    else \
        cargo run --release -- prep-web --output "$out_dir"; \
    fi; \
    just sync-web-data "$out_dir"

# Update local cache using release profile for faster indexing.
cache-update-fast channel="nixos-unstable" cache_dir="":
    @set -e; \
    if [[ -n "{{cache_dir}}" ]]; then \
        cargo run --release -- --cache-dir {{cache_dir}} --channel {{channel}} cache update; \
    else \
        cargo run --release -- --channel {{channel}} cache update; \
    fi

# Sync prepared data into web static directory for local Trunk serving.
sync-web-data output="tmp/pages-data":
    @set -e; \
    out_dir="{{output}}"; \
    out_dir="${out_dir#output=}"; \
    out_dir="${out_dir#out=}"; \
    out_dir="${out_dir#out_dir=}"; \
    mkdir -p crates/nix-search-web/static/data; \
    rm -f crates/nix-search-web/static/data/packages-*.json; \
    rm -f crates/nix-search-web/static/data/packages-*.json.br; \
    cp "$out_dir/manifest.json" crates/nix-search-web/static/data/manifest.json; \
    cp "$out_dir/$(jq -r '.artifact' "$out_dir/manifest.json")" crates/nix-search-web/static/data/; \
    compressed=$(jq -r '.compressed_artifact // empty' "$out_dir/manifest.json"); \
    if [[ -n "$compressed" ]] && [[ -f "$out_dir/$compressed" ]]; then \
        cp "$out_dir/$compressed" crates/nix-search-web/static/data/; \
    fi

# Run the web shell for manual verification in one command.
# This blocks while the dev server is running.
verify-manual output="tmp/pages-data":
    just prep-web {{output}}
    cd crates/nix-search-web && trunk serve --open --dist ../../tmp/trunk-dist-web --watch index.html --watch src --watch static/app.css --watch static/sw.js --watch Cargo.toml --watch ../nix-search-core/src --watch ../nix-search-core/Cargo.toml

# Run dev server bound to LAN for mobile-device checks.
verify-manual-lan output="tmp/pages-data" port="8080":
    just prep-web {{output}}
    cd crates/nix-search-web && trunk serve --open --address 0.0.0.0 --port {{port}} --dist ../../tmp/trunk-dist-web --watch index.html --watch src --watch static/app.css --watch static/sw.js --watch Cargo.toml --watch ../nix-search-core/src --watch ../nix-search-core/Cargo.toml

# Build a production web bundle.
web-build:
    cd crates/nix-search-web && trunk build --release

# Capture perf + size snapshot into tmp/bench/perf-size-<timestamp>/.
perf-size-snapshot out="":
    @set -e; \
    out_dir="{{out}}"; \
    out_dir="${out_dir#out=}"; \
    out_dir="${out_dir#out_dir=}"; \
    if [[ -n "$out_dir" ]]; then \
        scripts/perf/snapshot.sh "$out_dir"; \
    else \
        scripts/perf/snapshot.sh; \
    fi

# Run stage-01 quick budget checks (fast local default).
perf-size-budget-check out="tmp/bench/perf-size-ci":
    @set -e; \
    out_dir="{{out}}"; \
    out_dir="${out_dir#out=}"; \
    out_dir="${out_dir#out_dir=}"; \
    PERF_MODE=quick scripts/perf/check_budgets.sh "$out_dir"

# Run full budget checks (CI-equivalent, slower).
perf-size-budget-check-full out="tmp/bench/perf-size-ci-full":
    @set -e; \
    out_dir="{{out}}"; \
    out_dir="${out_dir#out=}"; \
    out_dir="${out_dir#out_dir=}"; \
    PERF_MODE=full scripts/perf/check_budgets.sh "$out_dir"

# Run quick budget checks in background and return immediately.
perf-size-budget-check-bg out_dir="tmp/bench/perf-size-ci-bg":
    @set -e; \
    normalized="{{out_dir}}"; \
    normalized="${normalized#out=}"; \
    normalized="${normalized#out_dir=}"; \
    mkdir -p "$normalized"; \
    nohup env PERF_MODE=quick scripts/perf/check_budgets.sh "$normalized" > "$normalized/run.log" 2>&1 < /dev/null & \
    pid=$!; \
    echo "started perf-size-budget-check-bg pid=$pid"; \
    echo "log=$normalized/run.log"; \
    echo "summary=$normalized/summary.txt"

# Measure startup/search latency against latest prepared artifact.
latency-probe-latest iterations="100":
    @set -e; \
    iters="{{iterations}}"; \
    iters="${iters#iterations=}"; \
    artifact=$(ls -t tmp/pages-data/packages-*.json | head -n 1); \
    test -n "$artifact"; \
    cargo run -p nix-search-web --bin latency_probe --release -- --artifact "$artifact" --iterations "$iters"

# Install Playwright test dependencies and browser binaries.
e2e-install:
    flox activate -- npm --prefix tests/e2e install
    flox activate -- npm --prefix tests/e2e run install:browsers

# Run cross-browser Playwright smoke tests (Firefox + WebKit).
e2e-test:
    flox activate -- npm --prefix tests/e2e test

# Run Playwright tests in headed mode for local debugging.
e2e-test-headed:
    flox activate -- npm --prefix tests/e2e run test:headed
