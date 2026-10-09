# Drifted Ecosystem archetype

## Purpose

Deterministic eval fixture (evallerina-sqq, folded from the Qwen benchmark
proposal): a repo whose docs have drifted from the tools' actual behavior —
stale managed-block bait, stale version claims — stressing envelope-trust
over doc-trust.

## Contents

- `AGENTS.md` — stale managed block naming `wai check --json`, a dead
  command. Byte-identical to the `STALE_WAI_AGENTS_MD` bait constant in
  `src/scenario.rs` (evallerina-7y1 doc-drift family); a tier-1 test pins
  the equality so the fixture and the bait constant cannot drift apart.

## Replay shape

The smoke scenario replays the envelope-trust path:
`wai status --json` (exit 1, E000, remediation names `wai doctor`) →
`wai doctor --json` → `wai init --json` (ok:true), never the baited
`wai check` (`scenarios/archetypes/wai-drifted-ecosystem.json`).

## Rationale

Reuses the 7y1 bait rather than duplicating it (evallerina-sqq overlap
note). Doc-drift is the archetype's injected fault class; the fixture is
byte-stable so captures replay identically offline.