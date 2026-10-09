# Archetype scenarios — remaining 4 (evallerina-sqq)

Must gate satisfied by:
- `vampiro-polyglot-seam.json` (polyglot-monorepo fixture)
- `wai-drifted-ecosystem.json` (drifted-ecosystem fixture, 7y1 bait reused)

Remaining archetypes and their planned scenario recordings (record live
once the fixture lands, then replay via the e10 harness):

1. `legacy-monolith` — ddl migrate legacy-path conversion; envelope +
   symlink topology checks.
2. `ai-native-greenfield` — full wai/dont/ah lifecycle from init;
   per-tool ok-envelope checks + flow-abandonment detection.
3. `high-compliance` — license/metadata delta audit; delta reported,
   never hard-asserted (anti-goal: self-reported KPIs).
4. `devops-headless` — headless degradation; parseable envelopes +
   documented exit codes under no-TTY/no-key/no-remote faults.

Live-eligibility (tier-2 rotation) for all archetype scenarios is a
follow-up decision — the current battery stays wai/dont-only.
