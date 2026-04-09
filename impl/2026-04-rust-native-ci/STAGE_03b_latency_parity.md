# Stage 3b — Latency probe + shell-parity harness

**Estimate:** ~6h
**Depends on:** Stage 3a
**User-visible change:** none; `ci budget` is now feature-complete locally.

## Goal

Add the latency-probe step to `steps::budgets` and land the end-to-end
parity harness that runs both `scripts/perf/check_budgets.sh` and
`ci budget` against the same seeded fixture. The parity test is the gate
for deleting the shell script in Stage 7d.

## Scope

### Latency probe

- Port the latency-probe shell-out from `scripts/perf/check_budgets.sh`.
- Keep the same binary / command invocation as today.
- Emit `p95_ms`, `p99_ms` (and any other fields the shell script records)
  into `BudgetReport`.
- Respect `BudgetConfig::run_latency_probe` (on for `full`, off for `quick`
  unless overridden).
- Heartbeat via `tracing` spans, same pattern as `trunk_build`.

### Shell-vs-Rust parity harness

- `tests/parity_vs_shell.rs`, gated `#[ignore]` by default so it does not
  run on every `cargo test` (it shells out to bash and needs the project
  toolchain).
- Seeds a tempdir with a fixture `crates/nix-search-web/static/data/`
  layout (manifest + artifact + compressed artifact, copied from a real
  `prep-web` run).
- Runs both:
  - `PERF_MODE=quick scripts/perf/check_budgets.sh <tempdir>/shell-out`
  - `ci budget --perf-mode quick --out <tempdir>/rs-out`
- Asserts:
  - Matching exit codes.
  - Matching numeric fields in `report.json` within tolerance:
    - Byte sizes: exact match if `brotli` crate+binary agree; otherwise
      ≤1% delta with a comment naming the deviating field. Document the
      tolerance decision in `WORKLOG.md`.
    - Latency fields: within ±2ms (probes are noisy).
  - Matching warn/fail decisions.
- Run the same parity test a second time with `PERF_MODE=full`.

### CI hook for the parity test

Add a one-off workflow `.github/workflows/ci-parity-once.yml`
(`workflow_dispatch` only) that runs `cargo test -p ci -- --ignored
parity_vs_shell` on `ubuntu-latest` with the same apt/trunk install the
legacy workflow uses. This gives us a reproducible green check before
cutting over in Stage 7d.

## Tests

- `latency_probe_unit.rs`: parse a canned latency-probe output blob and
  assert `BudgetReport` fields populate correctly.
- `parity_vs_shell.rs`: as described above, two runs (quick + full).

## Acceptance

- `cargo test -p ci -- --ignored parity_vs_shell` passes locally on
  maintainer's machine for both `quick` and `full`.
- Same test passes in the `ci-parity-once.yml` workflow run.
- Any byte-size tolerance above 0% is documented in `WORKLOG.md`.
- Stage 7d's "delete shell script" precondition is now satisfied.

## Out of scope

- Container image (Stage 4).
- Consumer workflow migration (Stage 5).

## Risks

- **Brotli byte-level drift** between the Rust crate and the `brotli`
  binary is the most likely parity failure. Mitigation: either switch
  `steps::budgets` to shell out to the `brotli` binary (sacrifices
  some purity for byte parity), or accept a small documented tolerance.
  Decision made during Stage 3b based on observed deltas.
- **Latency probe flakiness** — mitigated by ±2ms tolerance. If probes
  are noisier than that, increase the sample count in `full` mode or
  widen the tolerance with a justification comment.
