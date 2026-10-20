# Brief: evallerina-e4x — route audit faults through the filing path

Ticket: evallerina-e4x (P2, complexity:m, lane:afk). Feed doc-audit
`ERR_FEATURE_UNDOCUMENTED` / `ERR_DOC_NOT_FINDABLE` rows through the existing
fault routing (add-fault-ticket-routing machinery in `src/routing.rs`), per
openspec change `add-ecosystem-doc-evals` (tasks 3.1–3.2, design D3/no-new-
routing-semantics).

## Why
Small surface but high blast radius: it touches the filing path that can
create real tickets in real repos. Delegation with a hard TDD shape keeps
the lead lean while forcing the anti-goal tests (no filing without `--file`,
fingerprint dedup) to be pinned by failing tests first.

## Desired outcome
1. `src/doc_audit.rs` audit rows are accepted as fault rows by the routing
   layer: a row with code `ERR_FEATURE_UNDOCUMENTED` or
   `ERR_DOC_NOT_FINDABLE` routes like any other `ERR_*` agent-fault row —
   fingerprint token `evallerina:<scenario>:<fault-class>`, cross-model
   threshold `min(3, |registry|)` (existing propose() semantics reused
   verbatim — no new routing semantics per D3).
2. Tools with `upstream = true` in routing.toml (`bd`, `openspec`) emit
   `skipped`/`unmapped_upstream` proposal rows — never filed.
3. Dry-run by default: `evallerina doc-audit --dry-run` (or the existing
   propose output path) prints proposal JSONL; filing happens ONLY behind
   the existing `--file` flag; CI never passes `--file` (anti-goal pinned).
4. Fingerprint dedup not bypassable: duplicate tokens across runs collapse
   (existing dedup reused; a test proves double-proposing the same token
   yields one proposal).
5. Tasks 3.1: failing test — audit row crosses threshold → dry-run proposal
   with the fingerprint token. Task 3.2: dry-run smoke against the latest
   real doc-audit rows (`cargo run -- doc-audit` output) — proposals
   reviewed and committed as evidence, no filing.

## Exact files
- `src/routing.rs` — accept doc-audit rows (likely: map audit rows into
  fault-row shape feeding existing `propose()`; minimal diff)
- `src/doc_audit.rs` — only if a row-shape bridge function is needed there
- `src/main.rs` — CLI wiring: doc-audit dry-run proposal output path
- `tests/routing.rs` — failing tests first (threshold, upstream skip, dedup)
- NO changes to `report.rs` contract beyond what fok already landed

## Red/green sequence
1. RED: audit row with `ERR_FEATURE_UNDOCUMENTED` over threshold →
   `propose()` yields proposal with token
   `evallerina:<scenario>:ERR_FEATURE_UNDOCUMENTED`. GREEN.
2. RED: same for `ERR_DOC_NOT_FINDABLE`. GREEN.
3. RED: upstream=true tool's audit row → proposal kind
   `skipped`/`unmapped_upstream`, never a filing candidate. GREEN.
4. RED: dedup — same fingerprint token twice → one proposal. GREEN.
5. Wire CLI dry-run; run task-3.2 smoke against real rows; commit the
   proposal review as research (no filing).

## Out of scope
- No actual `bd create` filing in this ticket (73q owns validation)
- No new check-codes, no report_version changes
- No scenario/recording work (vrr/t7n)

## Anti-goals
- No filing without `--file` — pinned by test (CLI test: dry-run default)
- Fingerprint dedup must not be bypassed — pinned by test
- upstream=true tools are NEVER filed — pinned by test
- No new fingerprint token format (reuse existing machinery verbatim)

## Completion criteria
- `cargo test` exit 0
- `just tier0` exit 0
- `cargo run -- doc-audit` + dry-run proposal path → exit 0 with proposals
  JSONL written to `.wai/projects/evallerina/research/` as task-3.2 evidence
- openspec tasks 3.1–3.2 checked off

## Follow-up threshold
If the existing propose()/dedup machinery resists accepting audit rows
without semantic changes (>1h), STOP and record the impedance as research —
do NOT fork a second routing path.

## Verifier evidence contract
The verify session's review artifact MUST cite the artifacts it read and
include fresh test-run output (actual commands + verbatim tails) before
recording CONFIRM — otherwise REVIEW_REQUIRED.

## Spawn model
Implementor: `openrouter/z-ai/glm-5.3-flash` (matches PI_PROVIDER default);
context ceiling ≤ 50% of its window, computed at spawn.
