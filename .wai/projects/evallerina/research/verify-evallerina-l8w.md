# Verification artifact — evallerina-l8w (dry-run routing module)

Verifier: read-only session `subagent:evallerina-l8w:verify` (lead-side checks, single-model degradation path — no second model family spawn available).
Artifacts read (cited): `git log --oneline -8` / `git show --stat` output (below), `bd show evallerina-l8w`, `bd list --all`, `tests/routing.rs`, `src/routing.rs`, `src/report.rs`, fresh `cargo test --test routing` and `just tier0` runs.

## 1. Commits — MATCH

`git log --oneline -8` shows exactly 6 implementation commits on top of the
spawn record, one per brief red→green step:

```
52cabdb feat(routing): proposal schema + jsonl writer + loud unknown-scenario error (evallerina-l8w tasks 1.3, 1.7)
c39a557 feat(routing): routing.toml map + resolve (evallerina-l8w task 1.6)
78a843a test(routing): repo keyed on scenario recorded tool, row tool provenance-only (evallerina-l8w task 1.5)
95c1dfb test(routing): unmapped tool -> skipped(unmapped_tool), not fatal (evallerina-l8w task 1.4)
efa74c4 test(routing): agent faults (ERR_*) never routed (evallerina-l8w task 1.2)
615640d feat(routing): threshold filter + token builder, dry_run proposals (evallerina-l8w task 1.1)
```

`git show --stat 615640d`: src/routing.rs +284, tests/routing.rs +73,
src/lib.rs +1 (registration), Cargo.toml/Cargo.lock (+toml dep) — matches
claimed scope. Remaining commits touch only the three claimed files +
tests. Claimed file set (src/routing.rs, tests/routing.rs, routing.toml,
Cargo.toml) confirmed present and touched. Note: implementor's report
rendered two commit rows as "(next)/(last)" — actual hashes exist and
match the described task mapping; cosmetic reporting flaw only.

## 2. Tracker — MATCH

`bd show evallerina-l8w`: status IN_PROGRESS (◐), assignee charly vibes
(claimed atomically). Not closed. `bd list --all` agrees.

## 3. Tests — fresh runs pasted

```
$ cargo test --test routing
...
test unmapped_tool_is_skipped_not_fatal ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ just tier0        # (fmt + clippy + full test suite)
tier0-exit:0
tier0 exit 0 confirmed
```

12/12 routing tests pass; tier0 (fmt, clippy, 16+10+3 lib/integration
tests) exits 0.

## 4. Schema + agent-fault tests — MATCH

`tests/routing.rs:173` `fn every_proposal_row_carries_the_full_schema()`
and `tests/routing.rs:77` `fn agent_faults_are_never_routed()` both exist
and pass (included in the 12).

## 5. Threshold semantics — MATCH

`src/routing.rs:27` `const ROUTING_THRESHOLD: usize = 3;` and
`src/routing.rs:106` `let routing_threshold = configured_models.len().min(ROUTING_THRESHOLD);`
then `routing.rs:187` filters on `tally.models.len() >= routing_threshold`
— identical shape to `src/report.rs:26` `ROUTING_THRESHOLD = 3` and
report.rs's distinct-models tally. No private redefinition of the rule;
header doc (routing.rs:6) states the reuse explicitly.

## Verdict

**CONFIRM** — every claim cross-checked against reality; fresh test-run
output included above; artifacts cited. One cosmetic note (commit table
formatting in the implementor's report) is not a substance finding.
