# Stage 01: Measurement Harness and Release Fast Paths

## Objective
Make performance decisions reproducible and make obvious fast paths easy to use locally.

## Why First
Without stable numbers, later optimizations are hard to trust and easy to regress.

## Deliverables

1. Add `just` tasks for benchmark/size snapshots:
   - native command timings (`cache update`, `prep-web`, search run)
   - web artifact sizes (`wasm`, `js`, `data`) raw + gzip + brotli
2. Add explicit fast tasks using release binaries where appropriate:
   - `just prep-web-fast`
   - `just cache-update-fast`
3. Document when debug vs release should be used in local loops.
4. Store benchmark outputs under `tmp/bench/perf-size-<timestamp>/`.
5. Add initial CI budget checks for:
   - wasm raw and compressed size ceilings
   - primary data artifact compressed-size ceiling
   - upper bound for key command runtime on benchmark runner (warning-level first)

## Verification

- Running new `just` tasks produces parsable reports.
- Reports include before/after baseline for same machine.
- Existing workflows remain backward-compatible.
- CI budget checks run and report pass/fail/warn with clear thresholds.

## Estimated Effort
5-6 hours
