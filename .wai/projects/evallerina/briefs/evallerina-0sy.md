# Brief: evallerina-0sy — routing map parity gate

Ticket: evallerina-0sy (P1, complexity:m, lane:afk). Implement the tier-0
`routing-parity` check and expand `routing.toml` to the full ecosystem, per
openspec change `add-ecosystem-doc-evals` (tasks 1.1–1.4, design D2).

## Why
Bounded parser + gate wiring with a crisp TDD shape (parse block → diff map →
fail/warn); delegation keeps the lead lean. Independent value: after this
lands, `just tier0` fails the moment routing.toml drifts from ddl's
MANAGED-TOOLS block.

## Desired outcome
`routing_parity()` in a new `src/routing_parity.rs` (or routing.rs section —
new file preferred for the file-header convention) that:
1. Parses the MANAGED-TOOLS block + canonical-names table from
   `../dulce-de-leche/README.md` (tool list from the managed block only —
   the table deliberately lists only binary/crate name mismatches).
2. Resolves each tool's repo: canonical table → binary-name fallback →
   explicit routing.toml override. Third-party (`bd`, `openspec`) marked
   `upstream = true` in routing.toml.
3. Fails (exit non-zero / loud error row `ERR_ROUTING_PARITY`) on: unmapped
   first-party tool, unresolved repo path, upstream entry missing.
4. Warns (not fails) when the ddl sibling checkout is absent.
5. `routing.toml` expanded: testaruda, turu(whisper), pretender, vampiro,
   incitaciones, openspec (`upstream = true`), specodelic, funkalization,
   wild, dulce-de-leche, bd (`upstream = true`); genesis/espectacular/wai/dont
   already present.
6. GHA: tier-0 workflow gains a second `actions/checkout` for
   `charly-vibes/dulce-de-leche` into `../dulce-de-leche` so the gate
   enforces in CI.

## Exact files
- NEW `src/routing_parity.rs` — parser + resolver + check (MUST carry
  Purpose/Responsibilities/Rationale header — e10 Must gate)
- `src/lib.rs` — module registration
- `src/main.rs` — tier-0 subcommand wiring (`ERR_ROUTING_PARITY` reporting)
- `routing.toml` — 11 new/updated entries with `upstream` flags
- `tests/routing.rs` or NEW `tests/routing_parity.rs` — failing tests first
- `.github/workflows/` tier-0 workflow — sibling checkout

## Red/green sequence
1. RED: parser test — fixture README text (embed a string constant with a
   MANAGED-TOOLS block + canonical table) → expected tool→repo pairs incl.
   name-mismatch rows. GREEN.
2. RED: resolver test — tool absent from canonical table resolves to binary
   name; explicit routing.toml override wins. GREEN.
3. RED: gate test — remove an entry from a temp routing.toml → check fails
   naming the tool; unresolved path → fails; absent sibling → warning, exit 0.
   GREEN.
4. RED: upstream test — `bd`/`openspec` resolve but carry `upstream = true`.
   GREEN.
5. Wire main.rs + routing.toml expansion; `just tier0` green.

## Out of scope
- No filing-path changes (e4x/fok own those semantics; `upstream = true` is
  data only here)
- No scenario or report changes
- No ddl repo changes

## Anti-goals
- The gate must NOT silently skip first-party tools
- `upstream = true` entries are never filed (data + schema only; filing
  enforcement is e4x)
- Warning-degradation ONLY when the sibling genuinely absent — never paper
  over a parse failure

## Commands
- `cargo test` per step; `just tier0` at end
- Negative check by hand: comment out one routing.toml entry → `just tier0`
  fails naming it (then restore)

## Follow-up threshold
If the MANAGED-TOOLS block format resists robust parsing (>1h), STOP and
record the blocker as research instead of hardcoding fragile parsing.

## Spawn model
Implementor: `openrouter/z-ai/glm-5.3-flash` (matches PI_PROVIDER default);
context ceiling ≤ 50% of its window, computed at spawn.
