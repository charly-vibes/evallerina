# Brief: evallerina-bhh — make archetype scenarios live-eligible

Ticket: evallerina-bhh (P3, complexity:m). Add the two recorded archetype
scenarios to the tier-2 live battery (src/scenario.rs `live_scenarios()`),
so the rotation covers them, and mark the 4 remaining archetype fixtures as
explicitly out of scope (stub TODO unchanged, but the registry must be the
single source of truth).

## Why
Registry edit + eligibility checks are small and well-bounded; delegation
keeps the lead lean. Scope gate: the ticket says "once rotation cadence
proves stable" — rotation has run stably through this epic (o3j wired the
battery + artifact upload), so eligibility is now appropriate.

## Desired outcome
`live_scenarios()` returns 8 scenarios: the existing 6 + `polyglot_seam_scenario()`
+ `drifted_ecosystem_scenario()` (in src/scenario.rs already). Tests assert
the battery contains both archetype names and that live_scenario_by_name
resolves them.

## Exact files
- `src/scenario.rs` (extend live_scenarios + a test)
- `tests/evals_replay.rs` or scenario tests — wherever battery-shape tests live

## Red/green sequence
1. RED: battery contains 'vampiro-polyglot-seam' and 'wai-drifted-ecosystem';
   live_scenario_by_name resolves both. GREEN.
2. RED: rotation eligibility — every scenario in live_scenarios() has
   channel_of mapping (src/report.rs scenario_channel) so the dashboard
   never errors on unknown scenario. If polyglot/drifted lack channels, add
   mappings (archetype channel family). GREEN.

## Anti-goals
- The 4 stub fixtures (legacy-monolith, ai-native-greenfield,
  high-compliance, devops-headless) stay OUT of the battery — TODO.md is
  their home, do not register stubs
- No changes to recorded scenario JSON files
- No rotation threshold changes

## Commands
- `cargo test` per step; `just tier0` at end

## Follow-up threshold
If channel semantics resist archetype scenarios (>1h), STOP and record.

## Spawn model
Default implementor; context ceiling ≤ 50%.
