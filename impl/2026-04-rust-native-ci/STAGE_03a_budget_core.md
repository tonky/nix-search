# Stage 3a — Port `check_budgets.sh` core logic

**Estimate:** ~8h
**Depends on:** Stage 2
**User-visible change:** none (still no workflow touches; `ci budget` fully
runnable locally except latency probe)

## Goal

Replace the bulk of `scripts/perf/check_budgets.sh` (~264 lines of bash) with
a typed Rust implementation in `steps::budgets`: config, artifact checks,
trunk build wrapper, size measurement via the `brotli` crate, threshold
evaluation, and report rendering. The latency probe and shell-parity
harness land in Stage 3b. The old shell script stays on disk until
Stage 7d cleanup.

## Scope

### `BudgetConfig`

Typed struct with the same defaults as the shell script:

```
perf_mode: PerfMode                 // Quick | Full, from $PERF_MODE
run_trunk_build: bool               // $RUN_TRUNK_BUILD, auto from perf_mode
run_latency_probe: bool             // $RUN_LATENCY_PROBE, auto from perf_mode
wasm_max_raw: u64                   // 4_500_000
wasm_max_brotli: u64                // 800_000
js_max_raw: u64                     // 100_000
js_max_brotli: u64                  // 12_000
data_max_brotli: u64                // 2_600_000
brotli_quality: u32                 // 9
data_brotli_quality: u32            // 9 (full) | 4 (quick)
search_p95_warn_ms: u32             // 40 (shell default) — see delta note below
search_p95_fail_ms: u32             // 120 (full) | 0 (quick, meaning off)
```

`BudgetConfig::from_env()` reads env vars with the same names as the shell
script and applies the same fallback logic.

**Legacy env override delta.** The current `perf-size-budget.yml` sets
`SEARCH_P95_WARN_MS=45` and `SEARCH_P95_FAIL_MS=120` explicitly at the
workflow level, overriding the shell-script defaults of 40 and 120. The
Rust `BudgetConfig` defaults must match the shell script defaults (40 / 120),
and the new workflow in Stage 5a must pass the same overrides. A unit test
locks this in by asserting defaults match the shell-script values. Any
other env-var overrides from the legacy YAML must be carried verbatim into
Stage 5a.

### Steps ported from the shell script

Port each shell-script phase to a dedicated function in `steps::budgets`:

1. **Check prepared artifact inputs** — verify `manifest.json` and the
   artifact it points at exist. Fail with exit-code-2-equivalent error.
2. **Trunk build** (when `run_trunk_build`) — invoke
   `trunk build --release --dist ../../tmp/trunk-dist-budget` with heartbeat
   logging. Heartbeat is a `tracing` span emitting every 5s.
3. **Measure wasm/js sizes** — walk `tmp/trunk-dist-budget`, find the largest
   `*.wasm` and `*.js`, record raw bytes.
4. **Brotli-compress and measure** — use the `brotli` crate (avoids shelling
   to the `brotli` binary). Quality from `BudgetConfig`.
5. **Measure data artifact brotli size** — compress the artifact referenced
   by the manifest with `data_brotli_quality`.
6. **Latency probe** — **deferred to Stage 3b**. Stage 3a leaves the probe
   step as a stub that respects `run_latency_probe` but records zeros.
7. **Evaluate thresholds** — produce a `BudgetReport` and compare against
   `BudgetConfig`. Warnings vs failures match the current script.
8. **Render report** — write `report.json` and `report.md` to the out dir;
   if `$GITHUB_STEP_SUMMARY` is set, append the markdown there via `env::summary`.

### Wire-up

- `pipelines::budget()` replaces the Stage 2 stub with the real
  `steps::budgets`.
- `ci budget` accepts `--perf-mode`, reads other knobs from env (via
  `BudgetConfig::from_env`).

## Tests

- `budget_config_env.rs`: unit-test `from_env` for each env var, including
  the quick/full defaults and the invalid `PERF_MODE` case.
- `threshold_eval.rs`: feed synthetic `BudgetReport`s and assert
  warn/fail/pass outcomes across edge cases (exactly at threshold, just over,
  `fail_ms=0` meaning disabled).
- `report_render.rs`: `insta` snapshot of the markdown report for a fixed
  `BudgetReport`.
- `defaults_match_shell.rs`: asserts `BudgetConfig::default()` values match
  the literal defaults in `scripts/perf/check_budgets.sh` (40 / 120 /
  4_500_000 / etc.) — protects against silent drift while the shell script
  still exists.

## Acceptance

- `cargo test -p ci` passes.
- `cargo run -p ci -- budget --perf-mode quick` locally produces
  size/threshold report fields matching `PERF_MODE=quick
  scripts/perf/check_budgets.sh` (latency probe section excluded).
- Same for `--perf-mode full`.
- `defaults_match_shell` test passes.

## Out of scope

- Latency probe and full shell-parity harness (Stage 3b).
- Deleting the shell script (Stage 7d cleanup).
- Container image (Stage 4).
- Workflow changes (Stage 5).

## Risks

- **Heartbeat / progress output formatting** is not parity-checked. Only
  `report.json` numeric fields are checked for parity. Acceptable because
  log formatting is not load-bearing.
- **Brotli compression determinism**: the `brotli` crate at quality 9
  should produce byte-identical output to the `brotli` binary, but if the
  parity test flakes, relax the parity check to "within 1% byte size" and
  document the deviation here.
