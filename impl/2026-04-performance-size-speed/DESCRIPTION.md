# Performance, Size and Speed Improvements

## Problem Statement
Current workflows and artifacts are fast enough for iteration, but there is still material headroom in:

- Native command latency for `cache update` and `prep-web`
- Local index creation throughput (parse/group/index build path)
- Web startup payload and static artifact size (`.wasm`, `.js`, prepared data JSON)
- Repeatable benchmarking and regression detection

## Questions To Answer

1. Will release mode help in real workflows?
2. Can local index creation be parallelized safely and measurably?
3. Can `prep-web` be sped up and/or made lighter on CPU and network?
4. Which static file optimizations have the best ROI (`.wasm`, JS, data artifacts)?
5. Which improvements should become default local/dev and CI paths?

## Initial Findings (2026-04-09)

Measured locally in this workspace:

- `prep-web` binary runtime:
  - debug: `real 9.03s`
  - release: `real 2.05s`
  - approx speedup: `~4.4x`
- `cache update` on fresh cache dir:
  - debug: `real 5.61s`
  - release: `real 2.63s`
  - approx speedup: `~2.1x`
- Native binary size:
  - debug: `35 MB`
  - release: `9.9 MB`
- Current web static bundle (`tmp/trunk-dist-web`):
  - wasm: `4,192,901 B` (gzip `931,366 B`, brotli `661,696 B`)
  - js: `57,206 B` (gzip `9,567 B`, brotli `8,089 B`)
  - prepared data json: `24,913,134 B` (gzip `3,441,812 B`, brotli `2,247,291 B`)

These numbers strongly justify a dedicated optimization track.

## Scope

In scope:

- Native build/runtime tuning for local commands and CI
- Index creation and prep pipeline throughput improvements
- Web static file size reductions and transfer-time optimizations
- Instrumentation + benchmark automation for before/after deltas

Out of scope for this track:

- Product feature changes unrelated to performance/size
- Search relevance model redesign
