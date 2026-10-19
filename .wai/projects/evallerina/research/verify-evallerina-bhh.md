# Verify: evallerina-bhh — Archetype scenarios live-eligible

**Date:** 2026-10-19 · **Mode:** read-only verifier · **Ticket:** `evallerina-bhh` (IN_PROGRESS, P3)
**Implementor claims:** 61d3144 (register vampiro-polyglot-seam + wai-drifted-ecosystem in `live_scenarios`), 97e8431 (scenario_channel routes archetypes); 98 tests pass; tier0 exit 0.

## 1. Commits (git show --stat + diff read)

Read: `git show 61d3144`, `git show c4409f7`, `git diff 97e8431 c4409f7`.

- **61d3144** — `src/scenario.rs` (+7) and `tests/evals_replay.rs` (+64/-2). Diff read confirms:
  - `live_scenarios()` appends `polyglot_seam_scenario()` and `drifted_ecosystem_scenario()` after the 6 wai/dont entries.
  - New tests: `battery_registers_both_archetype_scenarios` (asserts len==8, both names present, both resolve via `live_scenario_by_name`) and `battery_excludes_stub_archetype_fixtures` (anti-goal guard over 4 stub keys).
- **97e8431 — ⚠ discrepancy:** `97e8431` exists as a commit object but is **NOT an ancestor of HEAD** (`git merge-base --is-ancestor` → false). It was amended into **c4409f7** (`evallerina-bhh: route archetype scenarios through scenario_channel`), which is HEAD~0 (HEAD = c4409f7). Diff `97e8431 → c4409f7` is only a rustfmt tidy in `tests/evals_replay.rs` (assert_eq line-wrapping, no semantic change). The substantive content (report.rs channel mappings + `archetype_scenarios_declare_channels` test) is identical.
- **c4409f7** — `src/report.rs` (+6) and `tests/report_aggregation.rs` (+23/-1), `tests/evals_replay.rs` (fmt-only). Diff read confirms `scenario_channel` gains:
  - `"vampiro-polyglot-seam" => Some("envelope")`
  - `"wai-drifted-ecosystem" => Some("artifact")`
- **No recorded JSON edits:** `git show --stat` for both ticket commits touches **only src/ + tests/** (61d3144: scenario.rs, evals_replay.rs; c4409f7: report.rs, report_aggregation.rs, evals_replay.rs). No `.json`, no recordings.
- **No threshold changes:** full `git show` output of both commits read; zero threshold-related lines. Broader `git diff 04c3fd8..HEAD` (ticket base) touches only src/, tests/, and `.wai/` chore files (spawn/claim/brief commits 8f346fe, f6ec665) — no threshold, no recordings.

## 2. Fresh runs

**`cargo test`** (fresh, exit 0) — per-suite:

| Suite | Result |
|---|---|
| unittests src/lib.rs | ok. 3 passed; 0 failed |
| unittests src/main.rs | ok. 0 passed; 0 failed |
| ablate_cli | ok. 4 passed; 0 failed |
| ablation | ok. 16 passed; 0 failed |
| evals_replay | ok. 18 passed; 0 failed |
| live_protocol | ok. 15 passed; 0 failed |
| report_aggregation | ok. 24 passed; 0 failed |
| routing | ok. 18 passed; 0 failed |
| doc-tests | ok. 0 passed; 0 failed |

Total = **98 passed, 0 failed** — matches implementor claim.

**`just tier0`** (fresh) — **exit: 0**. Log `/tmp/tier0-bhh.log`: recipe = `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`; 98 `... ok` lines, final suites all green.

## 3. Anti-goals

- **Stub fixtures absent from live_scenarios:** `grep -n 'legacy-monolith|ai-native-greenfield|high-compliance|devops-headless' src/scenario.rs` → **no matches (exit 1)**. ✔
- **No recorded JSON edits:** covered above — only src/ + tests/ touched by ticket commits. ✔
- **No threshold changes:** none in either commit's diff. ✔

## 4. Code state at HEAD (read paths)

`src/scenario.rs` (HEAD):
- L57 `pub fn live_scenarios() -> Vec<Scenario>` — L70–71 add `polyglot_seam_scenario()`, `drifted_ecosystem_scenario()` → **returns 8**.
- L77 `pub fn live_scenario_by_name(name: &str) -> Result<Scenario, String>` — resolves via `live_scenarios()`, so both archetype names resolve (pinned by the 18-passing evals_replay suite incl. `battery_registers_both_archetype_scenarios`).

`src/report.rs` (HEAD): `scenario_channel` maps both archetype names (envelope / artifact), pinned by `archetype_scenarios_declare_channels` (passing in report_aggregation) and guarded by pre-existing `every_live_scenario_declares_a_channel` (passing).

## 5. Ticket state

`bd show evallerina-bhh` → **P3 · IN_PROGRESS** (started 2026-10-19; lease-expired heartbeat is cosmetic).

## Verdict

**CONFIRM**

Registration + routing are deployed at HEAD, all 98 tests pass freshly, tier0 exits 0, anti-goals hold, no recorded-JSON or threshold tampering.

**Note (non-blocking):** the implementor cited hash `97e8431` for the routing commit, but that hash was amended into **`c4409f7`** (the deployed HEAD commit, same message; delta = rustfmt-only in one test file). Content claims hold exactly against `c4409f7`. The stale hash should be corrected in any close-out notes; the work itself is verified.
