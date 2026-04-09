# Stage 4 Worklog

- 2026-04-09: Added the container-image scaffold: `ci/Dockerfile`, `ci/image.sha`, and the GHCR build workflow.
- 2026-04-09: Added helper scripts for future image verification and manual pin bumps under `scripts/ci/`.
- 2026-04-09: Chose manual pin-file bumps for now; the first GHCR publish and package-visibility flip still need a maintainer bootstrap run.
- 2026-04-09: Switched the Dockerfile builder stage to Debian + rustup after the `rust:stable-slim` tag was unavailable in this environment.
- 2026-04-09: Local arm64 `docker buildx build` reached the Debian base pull but stalled in Orbstack, so the full image smoke test still needs a less constrained Docker runtime or GitHub Actions.
- 2026-04-09: Image size documentation is still pending until the first successful publish/build can complete.
- 2026-04-09: Podman build succeeded after increasing the Podman VM from 2 GiB to 6 GiB, and `podman run --rm localhost/nix-search-ci:podman doctor` passed.
- 2026-04-09: Podman is viable for this image path in this repo, but it needs VM tuning here; Docker/Orbstack still stalled on the base-image pull in the same environment.
- 2026-04-09: Public Orbstack issues show related reports for slow image downloads, docker builds stuck, and unreliable builds, which supports the conclusion that the stall is likely a platform/runtime issue rather than a repo-specific Dockerfile bug.
- 2026-04-09: Local validation will use Podman on Apple Silicon; GitHub Actions stays on Docker Buildx for multi-arch image publishing.
- 2026-04-09: Local Podman image size is 1.14 GB (`localhost/nix-search-ci:podman`), which is under the 1.5 GB compressed tracking target.
- 2026-04-09: GHCR bootstrap is still pending; `gh api /user/packages/container/nix-search-ci` returned 404 because the package has not been published yet.
