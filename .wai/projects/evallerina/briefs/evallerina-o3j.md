# Brief: evallerina-o3j — CLI --route flag + routed field + GHA artifact

Ticket: evallerina-o3j (P2, afk, complexity:m) — Phase 3 of openspec
`add-fault-ticket-routing`, tasks 3.1–3.2. Builds on src/routing.rs
(proposals, tasks 1.x) and its filing path (tasks 2.x, closed). Read
src/routing.rs, src/report.rs (aggregate_rows + scenario_channel),
src/main.rs, tests/report_aggregation.rs first.

## Why
Wiring is cross-cutting (CLI + report schema + CI) but each change is
mechanical; a subagent with the exact surface list avoids the lead
re-deriving the report contract.

## Desired outcome
`evallerina report --route [--file]` runs the routing step and records the
additive `routed` summary; the tier-2 GHA rotation uploads the dry-run
artifact; CI never passes `--file`.

## Exact files
- `src/main.rs` — add `--route` and `--file` flags to the report subcommand
  (`--file` requires `--route`; error otherwise)
- `src/report.rs` — call routing::propose after aggregation when `--route`;
  attach `routed` summary {filed, existing, dry_run, skipped} to the report
  JSON; write proposed-tickets.jsonl next to the report when routing ran
- `tests/report_aggregation.rs` — extend: routed field present after
  --route dry-run; report_version still 1; --file without --route is an
  error; absent rows never routed
- `.github/workflows/evals-rotation.yml` — upload proposed-tickets.jsonl as
  workflow artifact on scheduled runs; no `--file` anywhere

## Red/green sequence
1. RED: --file without --route → loud error. GREEN.
2. RED: report --route (dry-run) on a fabricated rows file → `routed`
   summary present with dry_run count > 0, report_version 1 unchanged,
   proposed-tickets.jsonl written. GREEN.
3. RED: report without --route → no `routed` field, no jsonl. GREEN.
4. RED: routing failures (unmapped tool) → skipped counted, report still
   succeeds. GREEN.
5. Edit evals-rotation.yml: add artifact upload step for
   proposed-tickets.jsonl. Grep-assert: no "--file" in the workflow.

## Commands
- `cargo test --test report_aggregation` per step; `cargo test` + `just
  tier0` green at end.

## Anti-goals
- routed field is additive; report_version stays 1
- no aggregate score field
- CI/scheduled workflows must never pass --file
- do not modify src/routing.rs except import/use

## Follow-up threshold
If the report JSON contract resists the additive field (>1h), STOP and
record findings.

## Spawn model
Model: sonnet or default. Context ceiling: ≤ 50% at spawn.
