# Brief: evallerina-l8w — dry-run routing module

Ticket: evallerina-l8w (P1, afk, complexity:m) — Phase 1 of openspec
`add-fault-ticket-routing`, tasks 1.1–1.7.

## Why
Delegation rationale: this is a self-contained TDD slice (new module +
tests, no report.rs surgery) that a fresh session can execute without the
orchestrator's context; the lead stays free to verify and orchestrate.

## Desired outcome
`src/routing.rs` + `tests/routing.rs` + `routing.toml` implementing the
dry-run routing step over tier-2 report rows.

## Exact files
- `src/routing.rs` (new — must carry a Purpose/Responsibilities/Rationale
  header; registered in `src/lib.rs`)
- `tests/routing.rs` (new)
- `routing.toml` (new: [wai] repo="../wai", [genesis] repo="../genesis")

## Red/green/refactor sequence (tasks 1.1–1.7)
1. RED 1.1: threshold test — fabricate report rows (serde_json) with a tool
   fault observed in trial rows from 3 distinct models → proposal row with
   token `evallerina:<scenario>:tool_fault`, decision `dry_run`.
   Reuse the report.rs semantics: threshold = min(3, |configured_models|),
   tally DISTINCT models per fault key (scenario, fault-class, tool) — do
   not redefine (see report.rs:72-86 for the rule).
2. GREEN: implement filter + token builder.
3. RED 1.2: agent faults (ERR_* prefixed) produce zero proposals → GREEN.
4. RED 1.4: unmapped tool → decision `skipped`, reason `unmapped_tool`,
   report still succeeds → GREEN.
5. RED 1.5: proposals key the repo on the scenario definition's recorded
   tool (scenario JSON `recorded.tool`), NOT the report row's `tool` field
   (hardcoded "wai" in live rows) → GREEN.
6. RED 1.6: routing.toml parse — relative paths resolve against the
   evallerina repo root via `git rev-parse --show-toplevel`; absolute paths
   pass through; missing mapping → skip path from 1.4 → GREEN.
7. RED 1.7: proposal row schema test — every row carries `tool`, `repo`,
   `scenario`, `fault_class`, `token`, `decision`, `reason` (null unless
   skipped), `bd_issue_id` (null in dry-run), `row_provenance`
   {scenario, model, tool} → GREEN.

## Public surface (keep minimal)
- `pub fn propose(rows, configured_models, scenario_tool_of) ->
  Result<Vec<ProposalRow>, String>` (names flexible; error paths loud,
  never silent buckets — mirrors report.rs conventions)

## Out of scope
- No `bd create`/`bd search` invocations (Phase 2 = evallerina-2bz)
- No CLI/`--route` flag or report wiring (Phase 3 = evallerina-o3j)
- No edits to src/report.rs beyond nothing; report_version untouched

## Commands
- `cargo test --test routing` (red → green each step)
- `just tier0` must stay green (fmt + clippy + all tests)

## Follow-up threshold
If any step reveals a design mismatch with report.rs >1 hour of work, STOP,
record findings in the ticket notes, and leave the ticket in_progress —
do not improvise a redesign.

## Spawn model
Model: sonnet (or configured default implementor). Context ceiling: ≤ 50%
of the model window at spawn time.
