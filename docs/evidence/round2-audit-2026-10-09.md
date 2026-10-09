Purpose/Responsibilities/Rationale
Responsibilities: round-2 audit corrections to docs/evidence/session-mining-2026-10-08.md — reproduces or refutes every contested headline number with committed scripts
Rationale: round-1 claimed "independently verified"; round-2 refutes that governance claim. All numbers below regenerate via `python3 <script>` from docs/evidence/ (routing.py, verify.py v3, adherence2.py, errors_deep.py)
---
# Round-2 audit: corrections to the 30-day session-mining report (2026-10-09)

Corpus: pi sessions under `~/.pi/agent/sessions/` for the 6 charly repos, 30-day
window (moving snapshot — counts drift upward while sessions accumulate; this run
was taken 2026-10-09 ~16:20 local). Scripts: `routing.py` (new), `verify.py` (v3,
rewritten after a mangled edit), `adherence2.py` (strict measure), `errors_deep.py`.

## Verdicts on round-1 headline claims

| Round-1 claim | Verdict | Corrected ground truth (this run) |
|---|---|---|
| "34.8% of 2,912 edits failed outright — verified" | **REFUTED (mixed population)** | `verify.py`: edit toolCalls=2,928, edit-tool errors=22 → **0.8%**. The old numerator matched VALIDATION in *any* toolResult: 826 bash + 180 read + ~26 edit. Reproducible as computed, mislabeled as an edit rate. |
| "ah ok:true + exit 1 in 36 of 308 successful checks (11.7%)" | **REFUTED (substring false positive)** | The 36 are *bash* toolResults whose JSON payload merely contains the text "exit 1" — not process exit codes. The real anomaly is on the native tool: `verify.py` v3: of 12 native `ah_check` toolResults, **8 exited 1 with an `ok:true` envelope passthrough** (4 normal). Same drift bait, 8/12 not 36/308. |
| "native ah_check 12 vs bash 156+ → 89.3% bash" | **DIRECTION ✓, DENOMINATOR ✗** | `routing.py`: native toolCalls=12 / toolResults=12 (reproducible — toolCalls live as content items on assistant messages). Bash ah-CLI invocations = **978** (exact `ah check` = 888); routing = **1.2% native / 98.8% bash over 990 calls**. The "156" was the *failed* subset, and 89.3% does not reproduce from 156:12 (that is 92.9%). |
| "342 preamble sessions; 172 incitadas; 152 (88.4%) followed; 20 (11.6%) improvised" | **DOES NOT REPRODUCE** | `adherence2.py` (strict): 178 requested-skill sessions; followed (SKILL.md read or /skill: invoked) = 162 → **91.0%**; no-evidence = 16 (9.0%). Round-1's measure was tautological (reading SKILL.md auto-passed the heuristic). |
| "/skill:renew improvised in 10 of 106 sessions (9.3%)" | **ABSOLUTES ✓, RATE ✗** | renew totals 122: 108 read-skillfile + 4 read-incitaciones + 10 no-evidence. 10/122 = **8.2%**; 9.3% reproduces from no cited denominator. |
| "total 1,152 toolResult errors" | **STALE SNAPSHOT** | `errors_deep.py`: **1,378** isError toolResults (+226 in ~2 days). Per-repo edit errors total 270; the report's edit-channel 82 was a round-1 undercount. |
| "12 native ah_check uses" | **REPRODUCIBLE ✓** | 12 toolCalls / 12 toolResults (espectacular 2, dulce-de-leche 2, wai 8). Round-2's intermediate "refuted" was an artifact of grepping the wrong schema location. |
| Provenance cites commit 3b1ed41 | **DANGLING** | Object exists but is unreachable from any ref (`git cat-file -t` = commit; not in `git log --all`). Lines 65/68 of the round-1 report cite a lost commit. |
| `adherence.py` ah_json_exit counter | **DEAD CODE** | Accumulated at lines 9/37/38, never printed. Round-1 numbers attributed to it came from elsewhere (adherence2.py / hand-runs). |

## Decision on the 34.8% claim

Marked **not-reproducible-as-claimed** rather than re-run: the definition was wrong
(VALIDATION from bash/read mixed into an edit rate). The corrected definition is in
`verify.py` v3 and yields 22/2,928 = 0.8%. No definition-level re-run is needed —
edit-localization failure is a *small* channel, not the headline.

## What survives (and drives evallerina priorities)

- **Hint adherence remains the biggest measured gap**, but by the strict measure:
  98.8% bash routing around a native tool the AGENTS.md hint names; 8.2% renew
  improvisation; 16 strictly-unfollowed skill requests. e10 → ey7 ordering holds.
- **The ok:true + exit-1 envelope drift is real but smaller**: 8/12 native
  ah_check toolResults (all `ok:true` passthrough with exit 1). Still the natural
  contrived-failure bait for evallerina-7y1 — refile scope accordingly.
- Edit-localization failures: 22/2,928 (0.8%) — demote from headline channel.
- All downstream mentions in the round-1 report (Executive Summary bullet 3,
  Evidence bullets, Decision 3, verification-state lines) are patched inline with
  pointers to this document.

## Method notes

- `routing.py` schema finding: assistant toolCalls are content items
  (`type:'toolCall'`) on assistant messages, not standalone messages; toolName is
  only a field on toolResult messages. Any counter that greps message-level
  `toolCall` undercounts to 0.
- Moving-corpus caveat: every count in this repo is a snapshot; scripts should
  print their window. Round-1 vs round-2 deltas (e.g. 1,152 → 1,378) are corpus
  growth, not script bugs.
