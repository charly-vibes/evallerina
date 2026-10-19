# Proposal: add-fault-ticket-routing

## Why

Tier-2 live runs classify faults (`CheckOutcome::tool_fault`, per-channel fault
dashboard in `src/report.rs`) but the classification stops at the report: tool
faults *should* become tickets in the tool's repo (the fault taxonomy's stated
routing contract, README "Fault taxonomy"), yet today a human must read the
dashboard and file each ticket by hand. The value that made evallerina useful
so far (evallerina-2rr, -00n, -54f) all came from tickets landing in the
respective tools — routing should not depend on manual triage.

## What Changes

- New `src/routing.rs` module: maps tier-2 tool-fault rows to tickets in the
  tool's own repo via the `bd` CLI (beads runs embedded per-repo, so filing
  happens in the target repo's working directory).
- Tool → repo mapping lives in a committed `routing.toml` (tool name → repo
  path + `bd` id prefix). Defaults to sibling checkouts (`../wai`,
  `../genesis`, `../espectacular`); the suite's `.genesis/tools.toml` remains
  the source of truth for *tool* identity, routing.toml only maps to repos.
- Deduplication before filing: a stable fingerprint token
  (`evallerina:<scenario>:<fault-class>`) is embedded in every proposed
  ticket; before filing, the target repo is searched for an **open** issue
  with the same token (`bd search --status open` — closed issues are
  deliberately no match so a regressed fault is re-filed). Existing match →
  skip, record `existing`.
- `evallerina report --route` flag: dry-run by default (writes
  `proposed-tickets.jsonl` next to the report); actual filing requires
  `--route --file`. Scheduled GHA tier-2 runs stay dry-run and upload the
  proposals as workflow artifacts — filing is a locally authorized action.
- Report gains an additive `routed` summary field (count per decision:
  `filed` / `existing` / `dry_run` / `skipped`), where `skipped` covers
  both unmapped tools and `bd` failures in the target repo (reason
  recorded, never fatal). report_version 1 unchanged.
- Proposals key the target repository on the scenario's recorded `tool`
  field, not the report row's tool value (currently hardcoded in live rows
  and prone to attribution drift); a machine-checkable
  `proposed-tickets.jsonl` schema makes the CI artifact reviewable.

## Impact

- Affected specs: new capability `fault-ticket-routing`
- Affected code: `src/routing.rs` (new), `src/report.rs` (routing hook),
  `src/main.rs` (`--route` flag), `tests/routing.rs` (new), GHA tier-2
  workflow (dry-run artifact upload)
