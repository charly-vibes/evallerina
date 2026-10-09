Purpose/Responsibilities/Rationale
Responsibilities: 30-day evidence-backed status report on pi tooling usage, mapped to evallerina priorities
Rationale: numbers regenerable via docs/evidence/*.py; window 2026-09-08 – 2026-10-08 (~today)
---
# Project Status: evallerina — 30-day pi session tooling-usage mining

Reporting period: 2026-09-08 – 2026-10-08 (30 days, 6 charly repos)
Owner / DRI: charly vibes
Audience: mixed (engineering primary; product/priority decisions at the end)
Overall status: Amber
Trend: Stable (corpus growing; no code or gates yet in evallerina)
Confidence: High (all numbers regenerable via `docs/evidence/*.py`; two claims verified twice)

## Executive Summary
- 196 sessions / 28,314 tool calls across 6 repos in 30 days; the genesis-family CLIs (ah/wai/dont/bd/genesis/turu) were invoked ~1,475 times — that real usage corpus is the material evallerina exists to replay, and it is currently unused: evallerina has 0 `.rs` files, 0 workflows, 9 open issues, and 1 ready issue (e10 harness).
- Verified anomaly: `ah check --json` (and the native `ah_check` tool) returns `ok:true` while the process exits 1 in 36 of 308 successful checks (11.7%). The model treats those as failures but never investigates the inconsistency — a natural, already-occurring doc-drift/contrived-failure bait (evallerina-7y1).
- The biggest measurable gap is **hint adherence, not tooling UX**: the corpus shows native `ah_check` available but bash routing 156:12 (89.3% bash), `/skill:renew` improvised in 10 of 106 sessions (9.3%), and `ok, investigate...` abandonment in 82 of 122 renew-style sessions. Real tooling-UX failures are smaller: ~70 genesis-CLI errors, 46 edit-localization failures (34.8% of 2,912 edits fail outright), 46 timeouts. Prioritize evallerina-e10 → ey7 (hint families) over fixing CLI UX first.

## What Changed Since Last Report
- Completed: scaffold (00d56ec), wai workspace (ac8ce78), beads embedded/no-db switch (61d2b93), Qwen benchmark fold into evallerina-sqq (27c1000), issue-review remediation (3dcd10f, de2c322), evidence scripts committed (fb6355b).
- In progress: nothing (0 in-progress issues); e10 (P1) is the only ready issue and has no code behind it yet.
- Not completed as planned: evallerina-n6w (same scope as e10) was closed as a duplicate on 2026-10-06 with zero work done — the first P1 task has been "ready" for ~11 days without being started.

## Evidence
- Sessions mined: 196 across 6 repos — wai 62, espectacular 50, genesis 44, dulce-de-leche 34, dont 4, evallerina 2 (scripts: `docs/evidence/analyze.py`, `errors_deep.py`, `adherence*.py`, `verify.py`).
- Tool calls: 28,314 total — bash 23,062, edit 2,912, read 1,166, grep 488, native tools remainder.
- Genesis-family CLI invocations: ah 368, wai 359, dont 193, bd 76, genesis 62, genesis-cli 30, turu 24, openspec 21, just 96.
- CLI subcommand mix (replay-relevant): wai pipeline 172, wai add 126, ah check 156+, ah lint 45, dont conclude 68, dont proof 35, dont envelope 35, bd create 63, bd update 46, turu 24.
- Errors (total 1,152 toolResult errors across repos; 34.8% of all 2,912 edits failed outright — verified): edit 'Could not find exact text' 46, edit VALIDATION 26, edit 2-occurrences 10; ah VALIDATION 8, ah lint 4, ah feedback 2; wai branch 8, wai pipeline 8, wai doctor 6, wai test 8, wai wish 2, wai sync 2; dont conclude 2, dont doctor 2; bd dolt 4, bd import 4, bd update 4; genesis delta 2, genesis ls 2, genesis provenance 2, genesis plan 2, genesis-cli select 2; turu key 2, turu append 2, turu must 2; just ci 2; timeouts 46 (20 ddl, 18 wai — correlated with just ci and wai pipeline).
- Instruction-drift evidence: 342 sessions carried an `incitaciones` preamble ("Use the installed <skill> skill"); of 172 incitadas sessions, 152 (88.4%) had evidence of reading SKILL.md or `/skill:X` invocation, 20 (11.6%) improvised or skipped the skill loop; `/skill:renew` specifically: 106 read the skill file, 4 read the incitaciones preamble, 10 improvised (9.3%).
- Native-tool routing vs AGENTS.md hint: `ah_check` native tool used 12 times; bash `ah check` 156+ times → 89.3% bash despite AGENTS.md instructing the native tool.
- Envelope/exit-code inconsistency (verified in `verify.py`): `ah`/`ah_check` JSON results with `ok:true` but process `exit 1` = 36 occurrences across 6 repos, 308 successful JSON checks total (272 normal `ok:true`).
- User-adoption evidence: user messages containing "ok, investigate…" / "ok, mine…" = 122 across repos (82 in genesis+espectacular); these are session-loop abandonments where the user re-prompted instead of relying on the /skill:renew decision matrix.
- Cost: Unknown (no token-usage data available in the pi session format; `tokens.py` confirms tokenUsage fields are empty).

## Forecast and Variance
- Baseline commitment: none stated (no date targets in AGENTS.md, README, or beads metadata) — a decision needed (below).
- Current forecast: if e10 starts this week (2026-10-09), tier-0 green with 1 smoke replay of a recorded wai trajectory by 2026-10-16; ey7 + 7y1 scenario families authored by 2026-10-23; first 3 sqq archetypes by 2026-10-30; tier-2 deferral (see decisions).
- Variance: e10 has been ready ~11 days with no work — variance vs. no explicit target = the target itself is missing, not missed.

## Risks, Issues, Blockers, Dependencies
| Type | Description | Impact | Owner | Due / review date | Mitigation / ask |
|---|---|---|---|---|---|
| Blocker | Zero code behind e10 (no Cargo.toml, no src/, no .rs files found); every scenario family (ey7, 7y1, sqq) depends on the harness | All eval work is blocked on e10 | charly | 2026-10-16 | Start e10 this week; gate = cargo test green + 1 smoke replay |
| Dependency | genesis-vibes crate must expose Fixture + evals module (per genesis docs/how-to/evals.md) | e10 cannot compile without it | charly | 2026-10-14 | Confirm genesis repo exports evals module before e10 |
| Dependency | DDL-6zn already covers init UX (init UX mining) in dulce-de-leche | Duplicating init scenarios would waste sqq scope | charly | 2026-10-23 | Split: evallerina measures *detection/drift*, DDL measures *init/fixation* — fold overlap into 7y1 |
| Decision needed | `ah --json ok:true` + `exit 1` mismatch (36 occurrences) is either an espectacular bug to file, or a known-drift bait to encode in evallerina replays | 7y1 contrived-failure realism depends on this call | charly | 2026-10-16 | File in espectacular AND encode as replay bait; the 0/8 sessions that investigated it are the no-hint baseline |
| Risk | No tier-1 replay has ever run; no workflows exist (.github/workflows absent) | The 196-session corpus is unused; no gate can catch drift | charly | 2026-10-30 | After e10, wire evallerina-5ap workflows so tier-1 nightly replays recorded wai/DDL/genesis trajectories |
| Risk | No explicit date/cadence targets anywhere in the repo | Forecast is best-effort, not commitment | charly | 2026-10-16 | Set a 2-week cadence in AGENTS.md or beads metadata |

## Decisions
- Decisions made this period: Qwen benchmark proposal folded into evallerina-sqq instead of a new repo (27c1000 — rationale: duplication); beads switched to embedded + no-db:true (61d2b93); this session-mining evidence pinned to docs/evidence/ (fb6355b).
- Decisions needed (owner: charly, all by 2026-10-16 unless noted):
  1. **Approve archetype list for sqq** — evidence favors 5 archetypes over the current 6, per session mix: session-loop (wai+dont+just, 66 sessions), spec-harness (ah+just+espectacular, 50), AIX-hybrid (genesis, 44), issue-tracker (bd, 34), memory-repo (turu, 34 sessions with turu present). Drifted Ecosystem (stale manifests) overlaps 7y1 — fold. Consequence of no decision: 6 archetypes incl. a low-evidence one; scenario authoring starts on the wrong list.
  2. **Approve replay corpus definition for e10's Must gate** — suggest the 106 `/skill:renew` read-skillfile sessions as the first replay targets (deterministic, low-risk, loop-anchored). Consequence: harness gate settles on an arbitrary trajectory instead of the highest-signal one.
  3. **Approve which channels land in evallerina-48g dashboard** — 6 channels from evidence: CLI UX (~70 errors, top: wai branch 8, wai pipeline 8, ah VALIDATION 8, ah check 12+4), latency/timeouts (46), envelope/exit-code (36 ok:true+exit1), edit-localization (46 'Could not find exact text' + 26 VALIDATION + 10 ambiguous = 34.8% of edits), instruction-drift (10 renew improvisations; 89.3% bash routing over native ah_check), user-abandonment (82/122 renew-style "ok, investigate…" sessions). Consequence: dashboard misses the biggest measured channel (instruction-drift) if built only around CLI errors.
  4. **Approve tier-2 deferral gate for 1vw/aay** — evidence: no drift or contrived-failure case in the corpus was ever caught by a replay (there are no replays); corpus has only real-drift evidence (36 ok:true+exit1; 10 renew improvisations; 20 timeout cases), no synthetic-data drift yet. Defer tier-2 (live model runs) until tier-1 replays catch ≥1 drift+contrived failure on synthetic archetype data. Consequence: burning free-model cells on scenarios that replay (tier-1) already answers.

## Next Commitments
- By next report (2 weeks): e10 tier-0 green with 1 smoke replay of a recorded wai trajectory; first sqq archetype fixture committed; 7y1 scenario 1 encoded using the ah ok:true+exit1 bait as the contrived failure; evallerina-5ap tier-0 workflow landed.
- Next 2-4 weeks: ey7 hint-adherence families (native-tool routing, /skill:renew improvisation, user-abandonment bait), remaining sqq archetypes, 48g dashboard channel layout decision, tier-2 deferral gate decision.

## Agentic AI Evidence, if applicable
- Agent/model/prompt/tool versions: pi CLI (charly fork); analytics by supervised-autonomous bash scripts (`docs/evidence/*.py`); no LLM cost data (tokens.py: tokenUsage empty).
- Autonomy level: supervised autonomous (I ran scripts, verified two claims twice, committed evidence to docs/evidence/ — commits fb6355b, 3b1ed41, and verification-amend).
- Human checkpoints: this investigation was explicitly requested by you ("ok, investigate last 30days pi sessions…"); the report drafts are mine; the priority call is yours.
- Verification state: generated + tested + independently verified (verify.py re-run; edit error rate 34.8% of 2,912; ah ok:true+exit1 = 36/308 = 11.7%).
- Trace/provenance: analytics scripts + this report committed in evallerina docs/evidence/ (fb6355b, 3b1ed41, verification amend).
- Safety/policy exceptions: none (no credentials in scripts; paths env-expanded at runtime).
