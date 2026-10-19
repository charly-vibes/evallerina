# Tasks: add-fault-ticket-routing

## 1. Routing module (dry-run first — red→green)

- [ ] 1.1 Failing test: `tests/routing.rs` — propose from a fabricated
  report with a tool fault ≥ threshold → `proposed-tickets.jsonl` rows with
  decision `dry_run`, correct token `evallerina:<scenario>:<fault-class>`
- [ ] 1.2 Failing test: agent faults (`ERR_*`) produce zero proposals
- [ ] 1.3 Implement `src/routing.rs`: threshold filter (reuse
  `min(3, registry)` rule from report.rs), token builder, JSONL writer
- [ ] 1.4 Failing test: unmapped tool → decision `skipped` + reason
- [ ] 1.5 Implement `routing.toml` parsing (serde) with relative-path
  resolution against the evallerina checkout

## 2. Filing path (separate red→green cycle)

- [ ] 2.1 Failing test: dedup — pre-seeded target repo with an open issue
  containing the token → decision `existing`, zero `bd create` calls
- [ ] 2.2 Implement target-repo search (`bd search <token>`, list+grep
  fallback)
- [ ] 2.3 Failing test: `--file` on a mapped tool invokes `bd create` in the
  target repo cwd (use a temp repo with beads initialized), captures issue id
- [ ] 2.4 Implement `bd create` invocation with exit-code + id capture

## 3. CLI + report wiring

- [ ] 3.1 `evallerina report --route [--file]` flag plumbing; default
  dry-run; `routed` summary field added to report (additive, report_version
  stays 1; update `tests/report_aggregation.rs` expectations)
- [ ] 3.2 Tier-2 GHA workflow: upload `proposed-tickets.jsonl` artifact on
  scheduled rotation; CI never passes `--file`

## 4. Validation

- [ ] 4.1 `just tier0` green (fmt/clippy/test)
- [ ] 4.2 `just tier1` replay unchanged (routing touches report post-processing
  only)
- [ ] 4.3 Manual smoke: dry-run against latest tier-2 rows produces sensible
  proposals (compare against the hand-filed evallerina-54f precedent)
