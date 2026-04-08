# Plan

1. Add diagnostics model and probe functions:
- Add a typed report for persistence/estimate/IDB write test outcomes.
- Reuse existing cache DB open path for realistic IDB verification.

2. Wire diagnostics into web shell:
- Add "Storage Diagnostics" action button.
- Run probe asynchronously and publish result into a small panel.
- Keep refresh/search behavior unchanged.

3. Validate and harden:
- Ensure probe failures degrade into readable text, not panics.
- Compile-check web crate.
