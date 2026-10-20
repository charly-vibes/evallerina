# Research: evallerina-fok — real-ecosystem doc-audit rows exceed follow-up threshold

Date: 2026-10-20 · Ticket: evallerina-fok · Trigger: brief follow-up threshold
("real-ecosystem audit rows exceed ~40 → STOP, record findings, ship mechanism
against fixtures, leave onboarding notes for 73q").

## Finding

`cargo run -- doc-audit` against the checked-in `doc_audit.toml` yields **91
audit rows** across 9 tools (threshold ~40). Breakdown:

- `ERR_DOC_NOT_FINDABLE`: 77 rows — token documented on some surface but not
  reachable from llms.txt / AGENTS.md managed-block breadcrumbs. Dominant
  pattern: most repos document commands/flags in README/docs but have no
  breadcrumb channel (no llms.txt, or llms.txt that doesn't enumerate the
  feature). This is the *intended* signal of the gate, not extractor noise.
- `ERR_FEATURE_UNDOCUMENTED`: 14 rows — e.g. `dont color/forget/nopersist/quiet/
  stats`, `turu quiet/verbose/help`, `specodelic verbose/exit1`, `ah help`,
  `vampiro completions`, `incitaciones dir`, `pretender help`.

Provenance (16/16 tools accounted, **zero silent skips**): 10 `extracted`,
4 `binary_absent` (genesis, espectacular, funkalization, dulce-de-leche —
binaries not on PATH in this checkout), 2 `skip` (bd, openspec —
`surface = none` by config).

No tool's `--help` resisted parsing — the general extractor rules (long flags,
Commands: sections, exit-code sections) harvested all 10 present binaries
without tool-specific cases. The threshold tripped on breadth (row count), not
parse resistance.

## Decision (per brief)

- Do NOT tune normalization to make real rows vanish (anti-goal).
- The mechanism ships against fixtures; all contract tests are hermetic
  (throwaway shell scripts + temp repos). `cargo test` green.
- Real-tool onboarding for 73q (validation phase): see notes below.
- The CLI exits 0 on findings — rows are the record (mirrors `report`
  precedent: findings do not affect the exit code; loud errors exit nonzero).
  Rows print on every `just tier0` run, so drift is surfaced immediately.

## Onboarding notes for 73q

1. **binary_absent tools**: build/install `genesis`, `espectacular`,
   `funkalization`, `dulce-de-leche` binaries on PATH (or
   extend `doc_audit.toml` with explicit `binary` paths into their
   `target/`) so their surfaces join the audit.
2. **NOT_FINDABLE triage**: 77 rows mostly mean "add an llms.txt breadcrumb or
   managed-block mention per feature". Recommend each repo owner add an
   llms.txt enumerating commands/flags; then rows collapse to genuinely
   undocumented tokens.
3. **Noise class**: clap auto-generates a `help` subcommand/flag; it shows up
   as UNDOCUMENTED when docs don't mention "help". If 73q wants to suppress
   it, that should be a *general* normalization decision recorded here — not
   per-tool tuning.
4. **Gate posture**: doc-audit rows do not fail `just tier0` (measurement,
   not gate). If the epic wants a gate later, add a threshold/scoped-gate
   decision to a follow-up ticket — do not silently tune the extractor.

## Artifacts read/produced

- Produced: `src/doc_audit.rs`, `doc_audit.toml`, `tests/doc_audit.rs`,
  report wiring in `src/report.rs`, CLI in `src/main.rs`, justfile tier-0 line.
- Fresh evidence: `cargo test doc_audit` (8 passed) and `just tier0` tails in
  the evallerina-fok ship record.
