# Stage 3b Worklog

- 2026-04-09: Started wiring the real latency probe into `crates/ci` and adding the shell parity harness.
- 2026-04-09: Planned parity tolerance: raw size fields exact, brotli fields allowed up to 1% drift, latency fields allowed up to 3ms drift.
- 2026-04-09: Switched brotli measurement to the external `brotli` binary for exact size parity.
- 2026-04-09: Observed startup latency noise above 2ms in parity runs; widened the harness tolerance to 3ms for latency fields.
- 2026-04-09: Validated with `cargo test -p ci`, `cargo test -p ci --test parity_vs_shell -- --ignored parity_vs_shell`, and `cargo build -p ci --release`.
