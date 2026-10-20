# Design: add-ecosystem-doc-evals

## Context

add-fault-ticket-routing already delivers the full loop: tier-2 report rows →
tool-fault filter → cross-model threshold (min(3, |registry|)) → fingerprint
token `evallerina:<scenario>:<fault-class>` → dry-run proposal rows → optional
authorized filing via `bd create` in the mapped repo cwd. This change does NOT
touch that machinery — it feeds it more scenario families and completes the
repo map so every fault has somewhere to go.

## Goals / Non-Goals

- Goals: doc-surface audits as tier-0 static checks; findability as replay/live
  scenarios; edge-case envelope-hint scoring; routing.toml == MANAGED-TOOLS
  parity with a drift gate.
- Non-Goals: aggregate doc-quality score (project anti-goal); mutating any
  tool's docs from evallerina; auto-filing tickets without `--file`.

## Decisions

### D1: Docs inventory is extracted, not hand-maintained

The tier-0 audit builds a per-tool inventory by scanning fixture-provisioned
repo surfaces (`README.md`, `docs/**`, `llms.txt`, `AGENTS.md` managed blocks)
for feature tokens harvested from the binary's observable surface
(`--help` parse: subcommands, long flags; plus documented exit codes from
`--help` where present). Alternative — a hand-written manifest per tool — was
rejected: it drifts exactly like the docs it audits and adds 15 files to sync.

Feature-token matching is substring-normalized (kebab/snake/spaced variants)
to avoid brittle exact-string asserts; unmatched tokens are the
`ERR_FEATURE_UNDOCUMENTED` rows. Tokens found in a docs file but not reachable
from llms.txt/managed-block breadcrumbs are `ERR_DOC_NOT_FINDABLE` rows.

### D2: routing.toml parity is gated, sourced from ddl

ddl's MANAGED-TOOLS block is the ecosystem source of truth (already enforced
inside ddl by `tests/tool_registry_drift.rs`). evallerina mirrors it: tier-0
check `routing-parity` parses the ddl repo checkout available in the workspace
sibling (`../dulce-de-leche/README.md` managed block) for the **tool list**, and
resolves each tool's repo from the canonical-names table when listed; tools not
in that table (the table deliberately lists only binary/crate name mismatches)
falls back to `repo == binary name`, overridable by an explicit routing.toml
entry. The check fails when a first-party tool lacks an entry or its repo path
does not resolve. **Third-party upstreams** (`bd` → gastownhall/beads,
`openspec` → fission-ai/openspec) get entries marked `upstream = true`: parity
requires the entry, but the filing path emits `skipped`/`unmapped_upstream` —
we never raise tickets in repos we don't control. genesis and espectacular are
not ddl-managed but remain first-party routing targets via explicit entries.

### D3: Edge-exploration faults reuse the existing taxonomy verbatim

`ERR_EDGE_UNHANDLED` = the tool exited non-zero with no `hints` entry on an
input the docs never document → tool fault → routes to the tool repo.
`ERR_ENVELOPE_HINT_BLINDNESS` (the existing class — no new name is minted) =
the tool emitted a hint and the agent ignored/replayed the same failing command
→ agent fault → routes to AIX channel work in genesis (existing cross-model
threshold decides). No new routing semantics.

### D4: Fixture provisioning follows genesis evals-guidelines

Each new scenario ships a sandboxed Fixture (fresh temp dir, isolated HOME,
pre-provisioned binaries, network denied unless opted in) built with the
genesis-vibes `Fixture` builder, matching hint-adherence/doc-drift scenarios.
Provenance note in each scenario JSON references the ticket or corpus decision,
per the existing scenarios/ convention.

### D5: Recording order = adoption order

Record live first for turu, testaruda, ah (recommended tier), then extension
tools as fixture bandwidth allows. Each recording lands as: live tier-2 run →
AgentStep transcript captured → scenario JSON + replay added to the tier-1
battery in the same commit (no orphan recordings).

## Risks / Trade-offs

- **`--help` parsing is heuristic** → mitigated by clippy-grade tolerance:
  audit rows are warnings-level findings that become faults only via the
  cross-model threshold, same as every other family; a noisy parser shows up
  in dry-run review before it can file anything.
- **Absent binaries in CI** → an audit row whose tool binary cannot be
  provisioned offline records `binary_absent` explicitly in provenance —
  never a silent skip (project anti-goal: absent cells must be explicit).
  Recordings target only tools with a reproducible offline install path
  (cargo install from lockfile or prebuilt release asset).
- **Sibling-repo dependency (D2)** assumes the ddl checkout sits next to
  evallerina in the workspace — true for this workspace; the gate degrades to
  a warning (not a failure) when the sibling is absent so standalone clones
  still pass tier-0. GHA workflows MUST check out the ddl sibling (second
  `actions/checkout` step) so the gate actually enforces in CI.
- **OpenSpec/mixed-ecosystem tools** (openspec, bd are not Rust CLIs) → the
  audit's binary surface step is skipped for tools whose fixture declares
  `surface: none`; their coverage rows come from the docs inventory only.

## Migration Plan

1. Ship the tier-0 audit + routing-parity gate (pure additions, no battery
   change).
2. Land routing.toml entries for all managed tools; dry-run review of any
   backfilled proposals against existing tier-2 rows.
3. Record turu/testaruda/ah scenarios; extend battery.
4. Archive after one full nightly rotation runs green with the new families.

## Open Questions

None. `ERR_DOC_NOT_FINDABLE` routes to the tool repo (doc-authoring fault):
channel plumbing faults already have `ERR_ENVELOPE_HINT_BLINDNESS` in genesis,
and a feature missing from breadcrumb channels is the tool's doc debt.
