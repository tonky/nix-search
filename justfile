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
    manifest="{{output}}/manifest.json"; \
    if [[ "{{force}}" != "1" ]] && [[ -f "$manifest" ]]; then \
        artifact="{{output}}/$(jq -r '.artifact' "$manifest")"; \
        if [[ -f "$artifact" ]]; then \
            echo "prep-web: using cached artifact $artifact"; \
        else \
            echo "prep-web: manifest exists but artifact missing, regenerating"; \
            cargo run -- prep-web --output {{output}}; \
        fi; \
    else \
        cargo run -- prep-web --output {{output}}; \
    fi
    just sync-web-data {{output}}

# Sync prepared data into web static directory for local Trunk serving.
sync-web-data output="tmp/pages-data":
    mkdir -p crates/nix-search-web/static/data
    rm -f crates/nix-search-web/static/data/packages-*.json
    cp {{output}}/manifest.json crates/nix-search-web/static/data/manifest.json
    cp {{output}}/$(jq -r '.artifact' {{output}}/manifest.json) crates/nix-search-web/static/data/

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

# Measure startup/search latency against latest prepared artifact.
latency-probe-latest iterations="100":
    artifact=$(ls -t tmp/pages-data/packages-*.json | head -n 1); \
    test -n "$artifact"; \
    cargo run -p nix-search-web --bin latency_probe --release -- --artifact "$artifact" --iterations {{iterations}}

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
