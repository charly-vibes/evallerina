# Brief: evallerina-wdx — 'evallerina ablate' CLI subcommand

Ticket: evallerina-wdx (P3, complexity:m). Expose src/ablation.rs
run_ablation via a CLI subcommand.

## Why
Mechanical CLI surface work, well-bounded; delegation keeps the lead
context lean.

## Desired outcome
`evallerina ablate <SCENARIO> [OPTIONS]` runs the A/B ablation over the
configured model registry and writes an ArmReport JSONL + summary.

## Exact files
- `src/main.rs` (new subcommand, mirroring existing subcommand patterns)
- `tests/ablation.rs` or a new `tests/ablate_cli.rs` (end-to-end via
  CARGO_BIN_EXE_evallerina in a temp sandbox)

## Red/green sequence
1. RED: `evallerina ablate --help` exits 0 and names the scenario arg.
   GREEN.
2. RED: end-to-end dry-run in a temp sandbox (no network, no model call —
   use an empty/absent registry path or a mock) writes the report files
   with ArmReport schema fields. Follow how run_ablation is invoked in
   tests today (see tests/ablation.rs run_ablation usage and its
   opt-in/opt-out real-generation flag from evallerina-0j8; wire the same
   flag through the CLI).
3. GREEN.

## Anti-goals
- No live model calls in tests; no network
- Do not modify src/ablation.rs beyond exposing the entry (pub fn already exists)
- Loud errors on bad scenario/registry paths, never silent

## Commands
- `cargo test --test ablation` + new CLI test per step; `just tier0` at end

## Follow-up threshold
If run_ablation's signature resists CLI wiring (>1h), STOP and record.

## Spawn model
Default implementor; context ceiling ≤ 50%.
