# Tasks: add-ecosystem-doc-evals

## 1. Routing map parity (no battery change)

- [ ] 1.1 Failing test: routing-parity parser — fixture README with a
  MANAGED-TOOLS block + canonical-names table → expected tool→repo pairs
- [ ] 1.2 Implement `routing_parity()` in `src/routing.rs` (or new module):
  parse sibling ddl README managed block, diff against routing.toml entries,
  unresolved paths fail, absent sibling → warning
- [ ] 1.3 Wire into tier-0 (`just tier0` / main.rs subcommand) and add
  `ERR_ROUTING_PARITY` reporting
- [ ] 1.4 Expand routing.toml: testaruda, turu(whisper), ah(espectacular
  exists), pretender, vampiro, incitaciones, openspec, specodelic,
  funkalization, wild, dulce-de-leche; verify parity check green

## 2. Tier-0 doc-coverage audit

- [ ] 2.1 Failing test: surface extraction — fixture binary `--help` with
  subcommands/flags → token set (kebab/snake/spacing normalization)
- [ ] 2.2 Implement surface extractor (`src/doc_audit.rs`: Purpose header per
  e10 Must gate); `surface: none` fixtures skip cleanly
- [ ] 2.3 Failing test: inventory extraction — fixture repo with README +
  docs/ + llms.txt + AGENTS.md managed block → feature-token set per surface
- [ ] 2.4 Implement inventory scanner; breadcrumb-reachability check
  (llms.txt / managed blocks) distinguishing ERR_FEATURE_UNDOCUMENTED vs
  ERR_DOC_NOT_FINDABLE
- [ ] 2.5 Wire audit rows into report as new check-codes (report_version
  stays 1, additive); `just tier0` green

## 3. Routing integration for audit faults

- [ ] 3.1 Failing test: audit row with ERR_FEATURE_UNDOCUMENTED crosses
  threshold → dry-run proposal with token `evallerina:<scenario>:<fault-class>`
- [ ] 3.2 Dry-run smoke against latest tier-2 rows; review proposals before
  any filing (mirrors 4.3 of add-fault-ticket-routing)

## 4. Doc-findability scenarios (first tools)

- [ ] 4.1 Fixture: turu (whisper) repo slice with llms.txt breadcrumb to a
  seeded feature doc
- [ ] 4.2 Record live tier-2 transcript; land scenario JSON + tier-1 replay in
  same commit (design D5)
- [ ] 4.3 Repeat for testaruda and ah; step budget + discovery-channel row
  provenance asserted in replay tests

## 5. Edge-exploration scenarios (first tools)

- [ ] 5.1 Failing test: classifier — non-zero exit without hints on
  undocumented input → ERR_EDGE_UNHANDLED; hint ignored → ERR_HINT_BLINDNESS
- [ ] 5.2 Fixtures + recordings for wai and dont edge inputs (existing battery
  tools first — cheapest provenance)
- [ ] 5.3 Extend to turu/testaruda/ah recordings as they land

## 6. Validation

- [ ] 6.1 `just tier0` green; routing-parity + doc-coverage gates pass on this
  repo itself (dogfood: evallerina audits its own docs)
- [ ] 6.2 `just tier1` replay unchanged for existing scenarios
- [ ] 6.3 One full nightly rotation green with new families; dry-run
  proposals reviewed; `--file` remains opt-in
