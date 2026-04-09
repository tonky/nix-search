# High-Level Plan

## Goal
Deliver measurable improvements in speed and size across CLI and web workflows, with repeatable benchmarks and safe staged rollout.

## Desired Outcomes

1. Faster local commands:
   - `cache update` and `prep-web` materially faster in default dev workflow.
2. Faster index creation:
   - Better throughput for parse/group/index build path.
3. Smaller/faster web artifacts:
   - Reduced wasm and payload transfer size.
   - Lower startup latency in browser.
4. Reliable measurement:
   - Bench + size budgets tracked to prevent regressions.

## Strategy

1. Establish a stable measurement baseline and default release-path shortcuts.
2. Optimize CPU-heavy native data/index steps with controlled parallelism and profiler-backed changes.
3. Optimize web payload format and transfer path first (largest ROI), then wasm/code size.
4. Lock in gains through CI checks, budget thresholds, and documented runbooks.

## Stage Handoff Contract

Each stage must produce explicit outputs consumed by the next stage:

1. Stage 01:
   - baseline timing + size reports
   - initial CI size/perf budget checks
2. Stage 02:
   - index throughput delta and determinism proof
3. Stage 03:
   - validated compressed/uncompressed artifact contract
4. Stage 04:
   - static/wasm size deltas and fallback compatibility status
5. Stage 05:
   - startup/search latency deltas and offline/cache behavior validation

## Success Metrics

- Native:
  - `cache update` real time reduction target: `>= 30%`
  - `prep-web` real time reduction target: `>= 35%`
- Web assets:
  - `.wasm` raw size reduction target: `>= 15%` or transfer-size reduction equivalent
  - primary data artifact transfer-size reduction target: `>= 25%`
- Startup/search UX:
  - startup read/hydration p50 reduction target: `>= 25%`
  - query p95 no regression relative to current baseline

## Risks

- Parallelism can change determinism or increase memory spikes.
- Aggressive size flags can hurt debugability or compile times.
- New payload encodings can increase complexity in offline/cache flows.

## Controls

- Keep each stage independently testable and benchmarked.
- Preserve deterministic output checksums where required.
- Use feature flags or fallback paths for risky format changes.
