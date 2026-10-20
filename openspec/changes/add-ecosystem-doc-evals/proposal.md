# Proposal: add-ecosystem-doc-evals

## Why

The battery currently exercises only **wai and dont**: 5 recorded scenarios plus
2 archetype recordings, and `routing.toml` maps only 4 repositories. The
dulce-de-leche suite manages **13 tools** (`ddl`'s MANAGED-TOOLS block is the
source of truth) — 11 of them (all but wai and dont), plus `ddl` itself, have
zero scenario coverage and zero ticket-routing targets. Tool faults found in
those tools today cannot route anywhere.

There is also no eval family that measures **the documentation as the product**:

1. **Coverage** — is every shipped CLI feature (subcommand, flag, exit code,
   envelope field) present in any doc surface (README, docs/, llms.txt,
   AGENTS.md managed blocks)?
2. **Findability/accessibility** — can an agent actually *locate* the doc for a
   feature through the AIX channels (llms.txt breadcrumbs, managed blocks)
   within a bounded step budget?
3. **Edge-case honesty** — when a fixture exercises undocumented or boundary
   inputs, does the tool emit a self-healing hint (`hints` envelope channel), or
   fail blind? Undocumented failures are doc faults, not just tool faults.

Each discovered fault becomes a fingerprinted proposal row routed to the
faulting tool's repo via the existing dry-run → `--file` machinery
(add-fault-ticket-routing Phase 2) — raising tickets to each repo mechanically
instead of by hand.

## What Changes

- **New capability `doc-coverage-evals`** with three scenario families:
  - `doc-coverage` (tier-0, static, no model): parse the fixture tool
    binary's observable surface (`--help`, subcommand tree, `--version`,
    documented exit codes) and diff it against a per-tool docs inventory
    extracted from the repo surfaces (README, docs/, `llms.txt`, AGENTS.md
    managed blocks). Missing/undocumented features emit
    `ERR_FEATURE_UNDOCUMENTED`; features documented nowhere findable emit
    `ERR_DOC_NOT_FINDABLE`.
  - `doc-findability` (tier-1 replay + tier-2 live): the agent must locate the
    documentation for a seeded feature via llms.txt / managed-block
    breadcrumbs; scored on process boundary (did the transcript reference the
    correct doc path, did the follow-up command succeed).
  - `edge-exploration` (tier-2): scenarios drive fixtures through documented
    boundary conditions *and* undocumented edge inputs; scoring asserts on the
    envelope's `hints` channel. Unknown-edge failures classify
    `ERR_EDGE_UNHANDLED` (tool fault) vs `ERR_ENVELOPE_HINT_BLINDNESS` (agent
    fault)
    exactly like the existing taxonomy.
- **Ecosystem-wide routing map**: `routing.toml` gains an entry for every tool
  in ddl's MANAGED-TOOLS block (+ `ddl` itself, `genesis`, `espectacular`).
  Third-party upstreams (`bd`, `openspec`) are marked `upstream = true`:
  parity requires their entry but the filing path never raises tickets in
  repos we don't control. A tier-0 drift check fails when routing.toml and
  the MANAGED-TOOLS block disagree (reuse of the registry-drift idea from
  ddl's `tests/tool_registry_drift.rs`).
- **Scenario recordings for the uncovered tools** start with the recommended
  tier (`turu`, `testaruda`, `ah`) before extension-tier tools, recorded live
  once fixtures land, then replayed per the tier ladder.

## Impact

- **Affected specs**: new capability `doc-coverage-evals` (this change);
  routing behaviour reused unchanged from add-fault-ticket-routing.
- **Report**: new fault classes appear as new `check-code` rows; matrix shape
  unchanged, `report_version` stays 1 (additive).
- **Fixture maintenance**: each additional tool adds a fixture (binaries,
  doc surfaces, recordings); recordings target only tools with a reproducible
  offline install path (see design Risks).
- **Non-goals**: no single doc-quality KPI score (anti-goal: per-capability
  breakdowns only); no scraping of upstream `fission-ai/openspec` docs beyond
  what ships in-repo; no automatic filing without the existing explicit
  `--file` authorization gate.
