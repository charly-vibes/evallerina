# Tasks: add-fault-ticket-routing

## 1. Routing module (dry-run first — red→green)

- [x] 1.1 Failing test: `tests/routing.rs` — propose from a fabricated
  report with a tool fault ≥ threshold → `proposed-tickets.jsonl` rows with
  decision `dry_run`, correct token `evallerina:<scenario>:<fault-class>`
- [x] 1.2 Failing test: agent faults (`ERR_*`) produce zero proposals
- [x] 1.3 Implement `src/routing.rs`: threshold filter (reuse
  `min(3, registry)` rule from report.rs), token builder, JSONL writer
- [x] 1.4 Failing test: unmapped tool → decision `skipped` + reason
- [x] 1.5 Failing test: proposals key the repo on the scenario's recorded
  tool, not the report row's `tool` field (hardcoded in live rows)
- [x] 1.6 Implement `routing.toml` parsing (serde); relative paths resolve
  against the evallerina repo root via `git rev-parse --show-toplevel`,
  absolute paths pass through (worktrees)
- [x] 1.7 Failing test: proposal rows carry the full schema (tool, repo,
  scenario, fault_class, token, decision, reason, bd_issue_id,
  row_provenance) — machine-checkable CI artifact

## 2. Filing path (separate red→green cycle)

- [x] 2.1 Failing test: dedup — temp dir with `bd init`, seed an open issue
  containing the token → decision `existing`, zero `bd create` calls
- [x] 2.2 Failing test: only a CLOSED issue contains the token → new issue
  proposed and filed (closed must not block re-filing a regressed fault)
- [x] 2.3 Implement target-repo search: `bd search --status open <token>`
  (list + grep fallback with the same status filter)
- [x] 2.4 Failing test: `--file` on a mapped tool invokes `bd create` in the
  target repo cwd (temp repo with beads initialized), captures issue id
- [x] 2.5 Implement `bd create` invocation with exit-code + id capture
- [x] 2.6 Failing test: `bd create` fails in target repo (non-beads dir) →
  decision `skipped` reason `bd_failure`, report still succeeds

## 3. CLI + report wiring

- [x] 3.1 `evallerina report --route [--file]` flag plumbing; default
  dry-run; `routed` summary field added to report (additive, report_version
  stays 1; update `tests/report_aggregation.rs` expectations)
- [x] 3.2 Threshold filter matches report.rs semantics: fault key observed
  in trial rows from ≥ min(3, |configured registry|) distinct models
- [x] 3.2 Tier-2 GHA workflow: upload `proposed-tickets.jsonl` artifact on
  scheduled rotation; CI never passes `--file`

## 4. Validation

- [ ] 4.1 `just tier0` green (fmt/clippy/test)
- [ ] 4.2 `just tier1` replay unchanged (routing touches report post-processing
  only)
- [ ] 4.3 Manual smoke: dry-run against latest tier-2 rows produces sensible
  proposals (compare against the hand-filed evallerina-54f precedent)
