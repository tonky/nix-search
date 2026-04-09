# Stage 4 — Container image with pre-baked `ci` binary

**Estimate:** ~9h
**Depends on:** Stage 1 only (can run in parallel with Stages 2, 3a, 3b to
front-load the highest-uncertainty infra work: multi-arch buildx, GHCR auth,
package visibility)
**User-visible change:** new image published to GHCR; no workflow consumers yet.

## Goal

Produce `ghcr.io/tonky/nix-search-ci:<sha>` containing the full toolchain and
a pre-built `/usr/local/bin/ci`. Ship a workflow that rebuilds and publishes
the image on changes to `crates/ci/**` or the Dockerfile. Verify the image
end-to-end by running `ci doctor` and `ci budget --perf-mode quick` inside it
against a checkout. Use Podman for local Apple Silicon smoke tests; keep
Docker Buildx for GitHub Actions.

## Scope

### `ci/Dockerfile`

Multi-stage:

1. **Builder stage** (`rust:1.<stable>-slim`): copy workspace, run
   `cargo build -p ci --release`. Use a cargo cache mount
   (`--mount=type=cache,target=/usr/local/cargo/registry`) if buildx is
   available.
2. **Runtime stage** (`debian:stable-slim`): install `trunk 0.21.14`, `jq`,
   `brotli`, `git`, `ca-certificates`, rust stable + `wasm32-unknown-unknown`
   target (needed because `ci budget` shells out to `cargo run -- prep-web`
   and `trunk build`, which compile project code at CI time). Copy
   `/usr/local/bin/ci` from the builder stage.
3. **Default ENTRYPOINT**: `ci` (so `docker run <image> doctor` works).

Multi-arch: build `linux/amd64,linux/arm64` via `docker buildx`.

### `.github/workflows/ci-image.yml`

- Triggers:
  - `push` to `main` when paths match `crates/ci/**`, `ci/Dockerfile`,
    `Cargo.lock`, or the workflow itself.
  - `workflow_dispatch`.
- **Permissions**: `contents: read`, `packages: write` (required to push
  to GHCR).
- Steps:
  - `actions/checkout@v6`
  - `docker/setup-qemu-action` + `docker/setup-buildx-action`
  - `docker/login-action` against `ghcr.io` (using `GITHUB_TOKEN`).
  - `docker/build-push-action` with `tags: ghcr.io/tonky/nix-search-ci:${{ github.sha }}`.
    **No `:latest` tag** — consumers pin by SHA.
  - Post-build: `docker run --rm ghcr.io/tonky/nix-search-ci:${{ github.sha }} doctor`
    as a smoke test.
  - Final step: update `ci/image.sha` pin file (see below) on `main` pushes
    only, via a bot commit back to `main`. On `workflow_dispatch` the
    pin update is left for a follow-up PR.

### SHA pin file

Add `ci/image.sha` — a single-line file containing the currently-consumed
image SHA. Both `perf-size-budget.yml` and `wasm-data-publish.yml` read it
via `$(cat ci/image.sha)` when constructing the `container:` reference.

Rationale: PLAN.md §4.5 promises "auto-PR bumping `<sha>` references" but
hand-rolling an auto-PR bot is out of scope. A single pin file is the
cheapest middle ground — bumping the image means one-line edit in one
file, either by the `ci-image.yml` workflow pushing to `main` or by a
maintainer opening a manual bump PR. Decision on automated push vs manual
PR is made during Stage 4 based on repo settings.

### GHCR package visibility

First push to GHCR creates the package as **private**. Consumer workflows
running in `container:` against a private GHCR image need `packages: read`
permission **and** a `docker/login-action` step before `actions/checkout`
— which GHA does not natively support since the container starts before
the first step runs.

Two options, pick one during Stage 4:

1. **Make the package public** after first push via
   `gh api --method PATCH /user/packages/container/nix-search-ci
    -f visibility=public`. Simplest; image contents are not sensitive
   (toolchain + open-source `ci` binary). **Recommended.**
2. Keep private and use a personal access token in a repo secret. More
   moving parts and PAT rotation risk.

### Bootstrap run

Explicit steps, assigned to the maintainer:

1. Merge Stage 4 PR to `main`.
2. `gh workflow run ci-image.yml` from `main`.
3. Verify the published tag with
   `docker pull ghcr.io/tonky/nix-search-ci:<sha>`.
4. Run the visibility fix (`gh api ... visibility=public`) once.
5. Commit the initial `ci/image.sha` with that SHA.
6. Confirm `docker run --rm ghcr.io/tonky/nix-search-ci:<sha> doctor`
   works locally and from a scratch `ubuntu-latest`
   `workflow_dispatch` runner.

### Drift-check helper

Add a tiny `scripts/ci/verify-image.sh` used in Stage 5:

```
#!/usr/bin/env bash
# Re-build ci from source inside the container and compare --version hash
# against the baked /usr/local/bin/ci. Exit 1 on mismatch.
```

Stage 4 only adds the script; Stage 5 wires it into the new PR workflow.

## Tests

- Dockerfile linted with `hadolint` (can run locally; no CI requirement).
- Local smoke: `podman build -f ci/Dockerfile -t nix-search-ci:local .` then
  `podman run --rm nix-search-ci:local doctor` on maintainer's Apple Silicon
  machine.
- GHCR smoke: after first push, `docker run --rm
  ghcr.io/tonky/nix-search-ci:<sha> doctor` on a plain `ubuntu-latest` runner
  via `workflow_dispatch`.

## Acceptance

- `ghcr.io/tonky/nix-search-ci:<sha>` pullable by the maintainer's account.
- Package visibility set (option 1 or 2 above, decision recorded in WORKLOG).
- `podman run --rm <image> doctor` exits 0 and prints sensible tool versions on
  the maintainer's local machine.
- **On a fresh `ubuntu-latest` runner (via `workflow_dispatch`)**: a job
  with `container: ghcr.io/tonky/nix-search-ci:<sha>` successfully
  checks out the repo and runs `ci doctor` and `ci budget --perf-mode
  quick`. This is the test for `container:` quirks (HOME, workdir, UID)
  that `docker run` on maintainer's machine does not cover.
- `docker run --rm -v $PWD:/w -w /w <image> budget --perf-mode quick`
  completes against a clean checkout on maintainer's machine.
- `ci/image.sha` committed to `main` pointing at a valid published tag.
- Image size documented in `WORKLOG.md` (target: <1.5 GB compressed;
  acceptable if larger, but track it).
- Image builds for both `linux/amd64` and `linux/arm64`.

## Out of scope

- Migrating consumer workflows (Stage 5).
- Automated image bump PRs (follow-up; manual bumps are fine for now).

## Risks

- **Rust toolchain duplication**: both builder and runtime stages need rust,
  so the runtime stage is bigger than ideal. Acceptable: alternative
  (shelling out to a pre-installed project toolchain) breaks the
  "single image" invariant.
- **`trunk` pin drift**: pin exactly `trunk==0.21.14` in the Dockerfile.
  Version bumps go through a normal PR touching the Dockerfile.
