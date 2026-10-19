# Design: add-fault-ticket-routing

## Context

Fault routing already has a normative statement (README fault taxonomy:
"tool faults become tickets in the tool's repo") and empirical precedent
(evallerina-2rr/00n/54f were hand-filed from dashboard findings). The missing
piece is mechanical routing from tier-2 rows to `bd` issues.

## Goals / Non-Goals

- Goals: tool faults from tier-2 rows become `bd` tickets in the tool's repo,
  deduplicated, with provenance back to the scenario and report row.
- Non-Goals: agent faults are never routed (they inform AIX work in genesis,
  per the taxonomy — that is a human decision); no cross-repo DB writes
  beyond `bd create` in the target repo; no GitHub issue filing (beads is the
  source of truth in every family repo).

## Decisions

### 1. Filing via `bd create` in the target repo working directory

beads runs embedded with `no-db: true` per repo; the only correct way to file
into `../wai` is to run `bd create` with that repo as cwd. Token budget is
irrelevant (no model call); exit-code checking + parsed issue id is captured
for the `routed` summary. Process-boundary scoring principle applies to
ourselves: we assert on exit codes and output, never on `bd` internals.

### 2. Deduplication by fingerprint token, searched in the target repo

A fault is (scenario, fault-class, tool). The token
`evallerina:<scenario>:<fault-class>` goes in the ticket body (the tool
version goes in the body too, not the token — a recurring fault on a new
version keeps the same token). Before filing: exactly
`bd search --status open <token>` in the target repo (`bd list` + grep with
the same status filter as fallback). Closed issues are deliberately no
match: `bd search` includes closed issues by default, so the status filter
must be pinned, and a fault that regresses after its ticket was closed is
re-filed rather than suppressed. Any open match → decision `existing`. This mirrors how
evallerina-54f already propagated ("same drift class as evallerina-54f" in
espectacular's tracker) — the token makes that linkage machine-readable.

### 5. `bd` failure in a mapped repo is `skipped`, never fatal

Filing is an optimization over the report, not a gate: if `bd create` fails
in the target repository (not a beads repo, `bd` binary missing, repo
locked), the decision is `skipped` with reason `bd_failure`, the reason is
recorded in the proposal row, and the report completes. The dashboard
already surfaces the fault; a broken tracker must not crash the rotation or
mask a `filed` count that did not happen.

### 6. `proposed-tickets.jsonl` schema (machine-checkable)

One JSON object per proposal row:

```json
{"tool": "wai", "repo": "../wai", "scenario": "wai-hint-adherence-typo",
 "fault_class": "tool_fault", "token": "evallerina:wai-hint-adherence-typo:tool_fault",
 "decision": "dry_run|filed|existing|skipped", "reason": null,
 "bd_issue_id": null, "row_provenance": {"scenario": "...", "model": "...", "tool": "..."}}
```

`reason` is null except for `skipped` (`unmapped_tool`, `bd_failure`);
`bd_issue_id` is set only for `filed`. The CI artifact is reviewable without
reading code.

### 3. Dry-run default, `--file` behind an explicit flag

Scheduled/CI runs must never file tickets (conservative authority policy).
`--route` alone produces `proposed-tickets.jsonl` (decision `dry_run` for
every row above threshold); `--route --file` additionally shells out to `bd`.
This keeps the nightly artifact reviewable before any cross-repo write.

### 4. Tool → repo mapping is a committed `routing.toml`

```toml
[wai]
repo = "../wai"
[genesis]
repo = "../genesis"
```

Paths resolve relative to the evallerina repo root (resolved via
`git rev-parse --show-toplevel`, so it survives worktree checkouts of
evallerina itself); absolute paths are accepted for target repos checked out
outside the family tree (e.g. worktrees — `ddl-worktrees` exists in this
family). Missing mapping for a tool → decision `skipped` with reason, never
an error (unknown tools must not break the rotation report). Target repo
resolution keys on the scenario definition's recorded `tool` field, not the
report row's `tool` value — live rows currently hardcode it (`src/live.rs`
`tool: "wai".into()`), and the hhs bug showed attribution can drift; the row
tool is carried in proposals for provenance only.

## Risks / Trade-offs

- `bd create` in a foreign repo mutates another project's tracker → mitigated
  by dry-run default + fingerprint dedup + `--file` opt-in.
- Tool attribution in live rows is currently hardcoded; routing keys on the
  scenario's recorded tool (Decision 4) and carries the row tool only as
  provenance, so attribution drift routes to the wrong ticket *text*, never
  the wrong repo.
- Rate: a bad nightly run could propose many tickets → the existing routing
  threshold (min(3, registry)) is reused so only faults reproducible across
  the configured minimum get proposed.
- Sibling checkouts may not exist in CI → `skipped`, artifact still uploaded.

## Migration Plan

1. Land module + dry-run path first (pure report side effect, no writes).
2. Wire `--file` + tests in a second commit (red→green per task ordering).
3. Update GHA tier-2 workflow to upload `proposed-tickets.jsonl`.

## Open Questions

- None. `--file` authority is granted by the human invoking it locally.
