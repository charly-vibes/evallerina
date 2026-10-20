# Brief: evallerina-fok — tier-0 doc-coverage audit

Ticket: evallerina-fok (P2, complexity:l, lane:afk). Implement the tier-0
doc-coverage audit (surface extractor + inventory scanner + audit rows), per
openspec change `add-ecosystem-doc-evals` (tasks 2.1–2.5, design D1/D4).

## Why
Bounded pure-Rust module with a crisp TDD shape (harvest tokens → scan docs →
emit rows) but large breadth: two extractors, a config format, and report
wiring. Delegation keeps the lead session lean for orchestrating the rest of
the epic (phases 3–6 depend on this landing). Independent value: after this
lands, `just tier0` flags undocumented or unfindable features across the
ecosystem the moment docs drift.

## Desired outcome
New `src/doc_audit.rs` (Purpose/Responsibilities/Rationale header — e10 Must
gate) providing:
1. **Surface extractor** — runs a tool's binary with `--help` (subprocess,
   in a provisioned fixture sandbox for tests) and harvests a feature-token
   set: subcommands, long flags, and documented exit codes where `--help`
   lists them. Token normalization: kebab/snake/spacing variants treated as
   equivalent (substring-normalized matching per D1).
2. **Inventory scanner** — scans a repo's docs surfaces (`README.md`,
   `docs/**`, `llms.txt`, `AGENTS.md` managed blocks) into per-surface
   feature-token sets.
3. **Breadcrumb reachability** — a token found in docs but NOT reachable
   from `llms.txt` or managed-block breadcrumbs → `ERR_DOC_NOT_FINDABLE`; a
   token absent from all surfaces → `ERR_FEATURE_UNDOCUMENTED`.
4. **Config** — a checked-in `doc_audit.toml` (tool → repo path, binary
   name, optional `surface = "none"`); `surface: none` tools (e.g. `bd`,
   `openspec` — not Rust CLIs) skip the binary step and are recorded in
   provenance, never silently skipped. Absent binaries record provenance
   `binary_absent` (never silent skip).
5. **Report wiring** — audit rows flow into the existing report as new
   check-codes `ERR_FEATURE_UNDOCUMENTED` / `ERR_DOC_NOT_FINDABLE`;
   `report_version` stays 1 (additive); `tests/report_aggregation.rs`
   expectations pass unchanged.
6. CLI: `evallerina doc-audit` subcommand; wired into `just tier0`.

## Exact files
- NEW `src/doc_audit.rs` — extractors + scanner + audit rows (MUST carry
  Purpose header)
- NEW `doc_audit.toml` — per-tool audit config (all routing.toml tools)
- `src/lib.rs` — module registration
- `src/main.rs` — `doc-audit` subcommand + tier-0 wiring
- `src/report.rs` — accept the two new check-codes (additive only)
- NEW `tests/doc_audit.rs` — surface + inventory contract tests (failing
  first); fixture binaries are throwaway shell scripts in a temp dir, NOT
  real ecosystem binaries

## Red/green sequence
1. RED: surface test — fixture script prints a `--help` with 2 subcommands +
   long flags → harvested token set matches, with a kebab/snake pair
   normalizing to one token. GREEN.
2. RED: `surface: none` test — tool declared `surface: none` produces no
   binary probe and records `skip` in provenance. GREEN.
3. RED: inventory test — fixture repo (temp dir: README + docs/x.md +
   llms.txt + AGENTS.md managed block) → per-surface token sets. GREEN.
4. RED: reachability test — token in `docs/x.md` not referenced by
   `llms.txt`/managed block → `ERR_DOC_NOT_FINDABLE`; token absent
   everywhere → `ERR_FEATURE_UNDOCUMENTED`; token reachable → no row. GREEN.
5. RED: `binary_absent` test — configured binary missing → provenance
   `binary_absent`, loud, not silent. GREEN.
6. Wire report rows (additive) + CLI + `doc_audit.toml` for the real
   ecosystem; `just tier0` green. Dogfood check (meter): audit run against
   evallerina's own surfaces produces a non-empty inventory with zero
   silent skips.

## Out of scope
- No routing/filing changes (e4x owns routing audit faults)
- No new scenario families or tier-2 recordings (vrr/t7n)
- No report_version bump, no aggregate doc-quality score (anti-goal)
- No edits to any sibling repo's docs

## Anti-goals
- Never silently skip a source: every skip is provenance (`skip`,
  `binary_absent`) — pinned by test
- No aggregate score field (pinned by test, same discipline as 48g)
- Token normalization must NOT be tuned to pass evallerina's own docs only
  (general normalization rules only)

## Commands
- `cargo test` per red/green step; `just tier0` at end; dogfood:
  `cargo run -- doc-audit` against evallerina itself → non-empty inventory,
  zero silent skips

## Follow-up threshold
If real-ecosystem audit rows exceed ~40 or the `--help` parsing of any real
tool resists the extractor (>1h), STOP: record findings as research, ship
the mechanism against fixtures, and leave real-tool onboarding notes for
73q (validation phase). Do not tune normalization to make real rows vanish.

## Verifier evidence contract
The verify session's review artifact MUST cite the artifacts it read and
include fresh `cargo test doc_audit` + `just tier0` output before recording
CONFIRM — otherwise REVIEW_REQUIRED.

## Spawn model
Implementor: `openrouter/z-ai/glm-5.3-flash` (matches PI_PROVIDER default);
context ceiling ≤ 50% of its window, computed at spawn.
