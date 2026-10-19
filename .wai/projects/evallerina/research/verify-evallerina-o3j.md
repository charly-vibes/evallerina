# Verification — evallerina-o3j (Route: CLI wiring + report routed field + GHA artifact)

Read-only verifier pass. Fresh commands executed from repo root
(`/var/home/sasha/para/areas/dev/gh/charly/evallerina`), HEAD = `adda629`.

## 1. Commits exist (git show --stat)

All five claimed commits are present at HEAD, in the claimed order:

```
=== 3676182 ===  report: --file requires --route (loud CLI error)
 src/main.rs                 | 15 +++++++++++++--
 src/report.rs               | 13 +++++++++++++
 tests/report_aggregation.rs | 12 ++++++++++++

=== d208a05 ===  report: --route runs the dry-run routing step, additive routed summary
 src/main.rs                 |  13 ++++-
 src/report.rs               |  92 ++++++++++++++++++++++++++++++++++
 tests/report_aggregation.rs | 118 ++++++++++++++++++++++++++++++++++++++++++++

=== c9795c9 ===  test: report without --route carries no routed field, no jsonl
 tests/report_aggregation.rs | 44 ++++++++++++++++++++++++++++++++++++++++++++

=== a4cb43b ===  test: unmapped tool counts skipped (report still succeeds); absent rows never routed
 tests/report_aggregation.rs | 74 ++++++++++++++++++++++++++++++++++++++++++-

=== adda629 ===  ci: upload proposed-tickets.jsonl dry-run artifact from rotation
 .github/workflows/evals-rotation.yml | 18 ++++++++++++++++++
```

Matches the claim: main.rs/report.rs wiring in the first two commits, 3 test
commits (+12 +118 +44 +74 = 248 test lines total), workflow-only CI commit.

## 2. Tests run fresh

### `cargo test --test report_aggregation`

```
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

23 passed — matches the claimed 17→23 growth. Routing-specific tests seen
passing: `route_dry_run_attaches_routed_summary_additively`,
`file_without_route_is_an_error`, `report_without_route_has_no_routed_field_and_no_jsonl`,
`report_route_dry_run_writes_proposed_tickets_jsonl`,
`unmapped_tool_counts_skipped_and_report_succeeds`,
`tool_fault_routes_to_tool_ticket_never_aix`,
`empty_configured_registry_is_an_error`, `unknown_scenario_channel_is_an_error`.

### `just tier0`

Exit code: **0**. All 8 test-result blocks show `0 failed`; totals 3+0+10+16+15+23+18+0.
Includes fmt (`cargo fmt --check`), clippy (`cargo clippy -- -D warnings`),
and the full `cargo test` suite. No failures/errors anywhere in output.

## 3. Workflow has no `--file`

`grep -n -- '--file' .github/workflows/evals-rotation.yml` → exit 1, no
matches. CI never passes `--file` (anti-goal satisfied). Workflow artifact
upload was added in `adda629` (+18 lines).

## 4. report_version / score invariants

From `src/report.rs`:
- `pub const DASHBOARD_VERSION: u32 = 1;` (line ~127) — **not bumped**.
- Row validation still enforces `report_version != 1` → error; dashboard emits
  `"report_version": report_versions.iter().next().copied().unwrap_or(1)`.
- The routed summary is decision counts only:

```rust
dashboard["routed"] = json!({
    "filed": filed,
    "existing": existing,
    "dry_run": dry_run,
    "skipped": skipped,
});
```

No aggregate score field added anywhere in the routed block or dashboard;
comments explicitly state "decision counts only, no aggregate score".
Additive-only confirmed.

## 5. Ticket status

`bd show evallerina-o3j` → `◐ evallerina-o3j · Route: CLI wiring + report routed field + GHA artifact  [P2 · IN_PROGRESS]`. Matches claim.

## Extras checked

- Working tree clean for src/, tests/, workflows/, .beads/ (only `.wai/` log
  files modified/untracked).
- Commit order topologically: `3676182 → d208a05 → c9795c9 → a4cb43b → adda629` (git log confirms).

## Verdict

**CONFIRM**

All five claims verified against fresh reality:
1. ✅ 5 commits exist with the described file footprints.
2. ✅ `cargo test --test report_aggregation`: 23 passed, 0 failed.
3. ✅ `just tier0`: exit 0, all suites green.
4. ✅ Workflow contains zero `--file` occurrences.
5. ✅ `report_version` stayed 1, `DASHBOARD_VERSION` not bumped, no score field added.
6. ✅ `bd show evallerina-o3j` is IN_PROGRESS.

No discrepancies found; no reasons for REVIEW_REQUIRED.
