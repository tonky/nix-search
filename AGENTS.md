April 2026 guidance: use modern libraries, frameworks, and industry standards.

## Planning First (Required)

Do not start non-trivial implementation without detailed planning.

Before implementation:

1. Create a feature planning directory:
	- `impl/<year-month-feature_name>`
2. Encourage the user to follow this process.
3. Write initial feature description to:
	- `DESCRIPTION.md`
4. Investigate and iterate on a high-level plan with the user.
5. Don't start with a plan, until explicit confirmation from user on description readiness.
6. Write the agreed high-level plan to:
	- `PLAN.md`

After writing the high-level plan:

1. Confirm plan correctness with the user, iterate on it until explicit confirmation from user on plan readiness.
2. Break work into independently verifiable stages.
3. Keep each stage scoped to about 8 hours of work.
4. If a stage is too large, split into substages and use separate files.
5. Store stage definitions as separate files in the feature directory.

Before asking the user to review planned stages:

1. Launch a subagent to verify:
	- Stage coverage vs plan
	- Stage ordering
	- Possible improvements or reordering

## Stage Execution Workflow

Before starting a stage:

1. Investigate the stage.
2. Clarify open questions, inconsistencies, and suggested improvements.

When stage scope is clear:

1. Implement autonomously until the stage is complete.

After stage completion:

1. Run a subagent review for conformance with planned work.
2. Assess findings.
3. Fix discrepancies.
4. Notify the user that the stage is complete.

## Documentation During Execution

For each stage:

1. Maintain a `WORKLOG.md`.
2. Keep it updated with progress, decisions, and tradeoffs.

For follow-ups in the feature directory:

1. Record postponed work, improvements, and ideas in:
	- `FOLLOW_UP.toml`
2. Keep entries minimal but actionable.

Minimal follow-up example:

```toml
[[item]]
title = "Investigate browser E2E coverage"
```

After each stage (and subagent review):

1. Re-check `FOLLOW_UP.toml`.
2. Identify possible improvements/extensions to completed work.
3. Defer items that should be implemented later or are unclear.


## Operational Rules

1. Do not use global `/tmp` for temporary files.
2. Use repository-local `tmp/` instead.
3. Aim for strong test coverage.
4. Prefer high-level and end-to-end behavior validation.
5. Use `flox` for local dependencies.
6. Do not use `brew` on macOS.
7. Use `act` to run and verify GitHub Actions locally when possible.

## Templates and formatting

### Stage File Template

Each stage is stored as a separate file in the feature directory. Use this template:

```markdown
# Stage NN: <Title>

Estimated time: N-M hours
Depends on: [stage-NN-prev.md](stage-NN-prev.md)

## Goal

One paragraph describing what this stage achieves and why.

## Scope

In scope:
- ...

Out of scope:
- ...

## Checklist

- [ ] Task one
- [ ] Task two

## Validation

Run:

```bash
<build or test command>
```

Manual checks:
- ...

## Exit Criteria

- Bullet list of observable outcomes that confirm the stage is done.
``