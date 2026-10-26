# Brief: evallerina-vrr — doc-findability tracer: turu (phase 4)

Ticket: evallerina-vrr (P3, complexity:l, lane:afk). First member of the
doc-findability eval family: a turu (whisper) fixture whose llms.txt
breadcrumb leads to a seeded feature doc, a live tier-2 recording of an
agent finding it, and a tier-1 replay in the SAME commit (design D5:
recording order = adoption order, no orphan recordings).

Openspec: `add-ecosystem-doc-evals` tasks 4.1 + 4.2 (NOT 4.3 — testaruda/ah
belong to evallerina-jzq). Spec: `specs/doc-coverage-evals/spec.md`
"Doc-findability eval family" requirement.

## Why
The doc-coverage audit (evallerina-fok/e4x) detects unreachable docs
statically; nothing yet measures whether an agent can actually FIND a
documented feature through breadcrumbs. This ticket lands the first
behavioral doc-findability scenario, establishing the family pattern
(fixture + recording + process-boundary scoring + tool-repo routing) that
jzq extends to testaruda and ah. Delegation keeps the lead session lean;
the mechanical part (scenario wiring, tests, YAML) is well-templated by
the seven existing scenario families.

## Desired outcome

1. **Fixture** (in-scenario, genesis `Fixture` builder — D4): a whisper
   repo slice materialized via `fixture_file`s:
   - `llms.txt` with a breadcrumb (a "Feature docs" style section) pointing
     to the seeded doc — deliberately NOT the first thing in the file; the
     agent must follow the breadcrumb.
   - The seeded feature doc, e.g. `docs/features/<slug>.md`, documenting a
     REAL turu capability that exit-0s hermetically in the trial sandbox
     (fixture root as cwd, isolated HOME, scrubbed env — PATH survives,
     src/live.rs:350). Verify empirically BEFORE committing to the
     capability: run the candidate command in a scratch non-git dir with
     `HOME=$(mktemp -d)` — `turu key --json` / `turu status --json` /
     `turu doctor --json` candidates; seed the doc around whichever
     succeeds cleanly (never fabricate behavior — the follow-up command
     failing would be a tool fault, not a findability signal). Record the
     verified command + why in the scenario JSON provenance.
2. **Scenario** (`src/scenario.rs`): `turu_doc_findability_scenario()`,
   family dir const `DOC_FINDABILITY_DIR = "scenarios/doc-findability"`,
   scenario name `turu-doc-findability-<slug>`. Prompt: repo slice is the
   workspace, llms.txt is the entry point, task = find the <feature> doc
   and run the documented command. Checks (process-boundary ONLY — anti-goal:
   never score free-form agent text):
   - `doc-path-referenced-within-budget`: some replay step's `command`
     references the exact seeded doc path within the step budget (≤ 6
     steps); fail = fault class `ERR_DOC_NOT_FINDABLE`.
   - `follow-up-command-succeeds`: the first turu command after the doc-path
     reference exits 0.
3. **Fault attribution** (the one real design seam): `CheckOutcome::Fail`
   carries no tool-fault code, and `CheckRow.code` is currently
   agent-fault-only (src/live.rs:158-171, scoring seam src/live.rs:686).
   REQUIRED observable: a failing doc-path check must produce
   `CheckRow.code = "ERR_DOC_NOT_FINDABLE"` (reuse
   `crate::doc_audit::ERR_DOC_NOT_FINDABLE`) with `taxonomy: None` (tool
   fault), so the EXISTING routing intake arm (src/routing.rs:179) routes
   it to the row's tool = turu → repo `../whisper` (routing.toml). Suggested
   mechanism (implementor may improve): evallerina-local
   `declared_fault_class(scenario_name, check_name) -> Option<&'static str>`
   consulted at the scoring seam — no genesis changes, no upstream edits.
   Update CheckRow's `code` doc comment additively (fault-class-carrying
   tool faults exist now). Test the full path: synthetic failed row →
   `evallerina report` → routing/propose DRY-RUN → proposal keyed
   turu/whisper, NEVER genesis/AIX (anti-goal, pinned by test).
4. **Channel**: `scenario_channel` gains the new name → `"docs"`
   (consistent with doc-audit — feature documentation is the channel).
5. **Rotation battery + provisioning**: add the scenario to
   `live_scenarios()` (required — `live <name>` resolves through it, and
   recording uses the harness) AND mirror the "Install eval targets" step
   in `.github/workflows/evals-rotation.yml` with
   `cargo install --locked --git https://github.com/charly-vibes/whisper`
   so Nov 2 rotation cells get an honest binary instead of 127s. Adding
   cells is safe: budget skips record as `Absent` (never failed), which is
   the honest record. If hermetic turu provisioning CANNOT be verified,
   STOP at the recording step instead of shipping an unprovisionable
   battery entry — record the blocker, keep the recording, file the
   follow-up.
6. **Live recording** (needs `OPENROUTER_API_KEY`, present in env):
   `cargo run --release -- live turu-doc-findability-<slug> --reps 1 --model
   openrouter/free --budget-mins 5` → extract the trial row's `replay`
   steps → transcribe into `scenarios/doc-findability/<name>.json` in the
   EXISTING recorded-JSON shape (name/provenance/recorded/steps with
   command/stdout/stderr/exit_code/executed — copy the
   `scenarios/doc-drift/*.json` shape; provenance names this ticket, the
   date, the model id verbatim, and the verified discovery channel =
   llms.txt-breadcrumb). If the first recording fails (agent genuinely
   can't find the doc = ERR_DOC_NOT_FINDABLE row), that is DATA, not a
   bug — retry once with a fresh run; if it fails again, land the FAILING
   recording's inversion? NO — the meter requires a passing replay; stop
   and report instead (two-strike rule).
7. **Tier-1 replay in the SAME commit as the JSON** (design D5): replay
   test in `tests/evals_replay.rs` running the recorded trajectory through
   the scenario — checks pass; plus the negative RED test (fresh-id /
   mutated transcript that never references the doc path →
   ERR_DOC_NOT_FINDABLE check fails).

## Completion criteria (runnable; exit 0 = done)
1. `cargo test evals_replay -- turu doc-findability` — the ticket meter:
   passes with the replayed transcript.
2. `cargo test` full suite green (tier-0 baseline unchanged).
3. `python3 -c "import json; json.load(open('scenarios/doc-findability/turu-doc-findability-<slug>.json'))"` —
   recorded JSON parses; shape matches doc-drift siblings (keys inspected
   by eye).
4. Routing dry-run evidence: `cargo run -- report <synthetic-failed-row.jsonl>`
   then propose dry-run shows a turu/whisper-keyed proposal carrying
   ERR_DOC_NOT_FINDABLE (command sequence recorded in the run report).
5. `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/evals-rotation.yml'))"`
   (if the install step was touched).
6. `just tier0` exit 0.

## Out of scope
- testaruda/ah fixtures + recordings (evallerina-jzq, task 4.3)
- Edge-exploration family (task 5.x, evallerina-t7n)
- Routing semantics changes (the routing.rs:179 arm already exists; reuse it)
- genesis upstream edits (fault-carrying tool faults stay evallerina-local)
- Any scoring of free-form agent text (commands + envelope channels only)
- Filing tickets (dry-run evidence only; `--file` stays untouched)

## Exact files
- `src/scenario.rs` (scenario + dir const + battery/channel registration)
- `src/live.rs` (fault-class seam at the CheckRow scoring point + doc comment)
- `src/report.rs` ONLY if scenario_channel needs the entry (it does — same
  file? scenario_channel lives in report.rs:137)
- `tests/evals_replay.rs` (+ routing test in `tests/routing.rs` for the
  ERR_DOC_NOT_FINDABLE → turu path if not already covered)
- `scenarios/doc-findability/<name>.json` (the recording)
- `.github/workflows/evals-rotation.yml` (turu install step, if battery entry)

## Red/green/refactor sequence
1. RED: replay test asserting the doc-findability scenario exists and a
   mutated transcript (no doc-path reference) fails with
   ERR_DOC_NOT_FINDABLE — fails to compile/link first (scenario absent).
2. GREEN: fixture + scenario + checks + fault-class seam; tests pass.
3. RECORD: live tier-2 run (criterion 6 above).
4. LAND: scenario JSON + replay tests in one commit (D5).
5. TIDY: separate commit for any doc-comment/clippy cleanup.

## Follow-up threshold
If no turu command exit-0s hermetically in the isolated sandbox, or two
live recordings both fail the findability checks, STOP and record the
blocker in the run report — do not fabricate a passing fixture or force a
third recording.

## Verifier evidence contract (wai-px4g)
The verify session must cite the artifacts it read (the diff, scenario JSON
at HEAD, the recorded row, fresh command output) and re-run completion
criteria 1–6 itself, including fresh `cargo test evals_replay -- turu
doc-findability` output, before recording CONFIRM — otherwise
REVIEW_REQUIRED.

## Spawn model
Implementor: z-ai/glm-5.3-flash via OpenRouter (free tier), the epic's
default; context ceiling ≤ 50% of the model window at spawn (flash-class
window ≥ 128k; this change is ~5 files plus one recording and will not
approach it). Verifier: cross-family mistral-small-3.2 per epic convention.
