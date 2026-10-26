# Brief: evallerina-xc3 — report fault codes by trial status class

Ticket: evallerina-xc3 (P3, complexity:s, lane:afk). Review finding EDGE-003.

## Why
The dashboard folds trial-level `invalid_output` faults
(ERR_ACTION_FORMAT_VIOLATION — agents that produced NO valid action, empty
replays) into the same channel/code space as hint-blindness faults from
`failed` trials (rich exploratory replays). Blending them overstates the
envelope-hint signal and corrupts the fault taxonomy that routes agent
faults to genesis AIX work. Mechanically verified from the Oct 12/19
artifacts: 3 invalid_output rows per run were narrated as hint-blindness.

## Desired outcome
Additive change to src/report.rs aggregation (dashboard_version unchanged —
fields added, none removed):

1. Every matrix entry (and routing entry) for AGENT faults gains a
   `status_class` field: `"failed"`, `"invalid_output"`, or `"http_error"`
   — the trial status class the fault came from. FaultKey::Agent gains the
   status class as a key dimension (or the tally carries it — implementer's
   choice, but the JSON must show it).
2. Trial-level `invalid_output` faults (the ERR_ACTION_FORMAT_VIOLATION
   branch) are keyed status_class="invalid_output" — NEVER blended into the
   failed-trial hint-blindness counts.
3. Check-row faults from failed trials (hint-blindness codes) key
   status_class="failed".
4. `http_error` transport rows never produce agent-fault entries (already
   true — keep pinned).
5. Anti-goals preserved: no aggregate score field; absent/rate_limited
   surfaced in coverage, never failures; unknown statuses loud.
6. AC-2 (summaries derive hint-blindness only from trials with non-empty
   replays) is SATISFIED BY the separation: hint-blindness counts = failed-
   class agent faults; invalid_output-class faults carry empty replays by
   definition. Add a comment in src/report.rs stating this derivation rule.

## Completion criteria (exit 0 = done)
1. `cargo test --test report_aggregation` green, including the new
   status-class separation test.
2. New test: one failed trial with hint-blindness code + one invalid_output
   trial (ERR_ACTION_FORMAT_VIOLATION) in the same channel → matrix/routing
   shows two distinct entries with distinct status_class values; the
   hint-blindness entry's occurrence count excludes the invalid_output one.
3. All pre-existing tests/report_aggregation.rs tests pass unchanged OR are
   updated ONLY if their JSON assertions break additively (justify each
   edit in the report — field additions should not break field-presence
   assertions).
4. `just tier0` green.

## Out of scope
- No CLI/dashboard consumer changes beyond the additive fields
- No changes to report_version or dashboard_version
- No routing-threshold semantics changes (distinct_models counting may
  need to happen per status class — pick the conservative reading: count
  distinct models WITHIN a status class, and say so in a comment)

## Exact files
- `src/report.rs` (FaultKey/tally/matrix/routing JSON + derivation comment)
- `tests/report_aggregation.rs` (new separation test first — RED)

## Red/green sequence
1. RED: write completion-criteria test 2 — assert two entries with
   distinct status_class; fails because the field doesn't exist.
2. GREEN: add status class to FaultKey::Agent + JSON emission.
3. Tidy: derivation-rule comment; confirm no assertion edits were needed.

## Follow-up threshold
If FaultKey changes ripple into routing thresholds or serde contracts
beyond additive JSON fields, STOP and record.

## Verifier evidence contract (wai-px4g)
Cite read artifact paths (>=1 .wai/ .md path) AND paste fresh test-run
output (actual command + result lines, e.g. "test result: ok. N passed") —
otherwise REVIEW_REQUIRED.

## Spawn model
Implementor z-ai/glm-5.3-flash (OpenRouter free); context ceiling ≤ 50%.
