# Brief: evallerina-vvt — schedule ablation cells in the weekly tier-2 rotation

Ticket: evallerina-vvt (P2, complexity:s, lane:afk). Slice (b) of the
evallerina-t3s split. Wire ablation cells into `.github/workflows/
evals-rotation.yml` so each weekly rotation produces `arm=ablated` rows next
to `arm=full` rows within the wall-clock budget.

## Why
The ablation harness (run_ablation, src/ablation.rs) and its CLI
(`evallerina ablate`, evallerina-wdx + evallerina-0j8 real artifacts) are
fully landed, but nothing invokes the CLI in CI — plain rotations default to
Arm::Full, so every rotation to date has zero ablation cells (all 144 rows
arm=full) and the A/B delta (the measured value of the AIX documentation
investment, a headline goal of the battery) is never produced. This is a
bounded single-YAML edit with no Rust changes; delegation keeps the lead
session lean for the remaining epic slices.

## Desired outcome
The `evals-rotation` workflow runs ablation cells AFTER the default registry
cells, in the same run, with all rows landing in
`reports/tier2-rotation.jsonl`:

1. New workflow step "Run ablation cells" between the rotation step and the
   aggregate step, gated `if: always()` so a failed/absent rotation never
   prevents ablation rows from being attempted or recorded:
   ```yaml
   cargo run --release -- ablate wai-hint-adherence-corrupt-config \
     --real-artifacts --budget-mins 5 --out reports/ablation-cells.jsonl
   cat reports/ablation-cells.jsonl | tee -a reports/tier2-rotation.jsonl
   ```
2. Budget split documented in the step comments: rotation `--budget-mins 15`
   (was 20) + ablation `--budget-mins 5` = the same 20-minute total, still
   under the 30-minute job ceiling with slack for the trial in flight.
3. `--real-artifacts` works on the runner because the existing
   "Install eval targets" step already puts `wai` on PATH — note this in a
   comment; the ablate CLI refuses to fake artifacts loudly if it's absent.
4. `--out` file cleaned/overwritten per run (File::create truncates; but
   ensure the `mkdir -p reports` happens before the ablation step too, or
   keep the single `mkdir` in the rotation step — verify ordering).
5. The existing persist step (evallerina-17o), dashboard aggregate step, and
   artifact uploads need NO changes: ablation rows flow through
   `reports/tier2-rotation.jsonl` and carry the additive `arm` field the
   report already understands (report_version 1 unchanged).
6. Anti-goal check: no aggregate score field appears anywhere; the ablate
   stdout summary (full/ablated/delta) is the only aggregated view.

## Completion criteria (runnable; exit 0 = done)
1. `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/evals-rotation.yml'))"` — YAML parses.
2. `actionlint .github/workflows/evals-rotation.yml` if available, else
   manual review: step names, `if: always()`, env scoping
   (OPENROUTER_API_KEY must be present on the ablation step too).
3. `just tier0` green (the YAML change must not break the tier-0 gate;
   doc-audit scans the repo).
4. Dry-run hermetic confirmation of the exact command line the workflow
   will run (no network):
   `cargo run -- ablate wai-hint-adherence-corrupt-config --reps 3 --dry-run --budget-mins 0 --out /tmp/vvt-ablation.jsonl && wc -l < /tmp/vvt-ablation.jsonl` —
   must exit 0 and produce 24 rows (2 arms × 3 reps × 4 models when
   budget-mins 0 records every cell absent... any row count ≥ 1 with both
   arms present is acceptable for the meter).

## Out of scope
- No Rust/CLI changes (the ablate subcommand is complete)
- No scheduling of ablation over the whole battery (one representative
  scenario per rotation keeps the budget honest; multi-scenario rotation
  design is a follow-up if the Nov 2 artifact shows appetite)
- No interpretation of the delta (evallerina-41g, DESIGN/HITL lane)
- Do not touch the persist step semantics (append-only, no empty commits)

## Exact files
- `.github/workflows/evals-rotation.yml` (the ONLY file this ticket changes)
- README/docs: only if the budget-split comment belongs in the workflow file
  itself (preferred) — do not open new doc channels

## Red/green sequence
This is a YAML-only change; the "red" is the current state (artifact of
Oct 12/19 rotations: 0 rows with arm != full). Verify each:
1. RED evidence: `grep -c "arm" reports/tier2-rotation.jsonl || true` — the
   committed rotation rows predate ablation; the workflow has no ablation
   step. State this in the report.
2. GREEN: apply the step insertion + budget split + comment updates.
3. Validate: completion criteria 1–4 above.

## Follow-up threshold
If the ablation step ordering resists (persist step racing the ablation
step, artifact upload ordering), STOP and record the conflict rather than
restructuring the workflow.

## Verifier evidence contract (wai-px4g)
The verify session must cite the artifacts it read (the diff, the workflow
file at HEAD, fresh command output) and include fresh test-run output
(completion criteria 1–4 re-run by the verifier) before recording CONFIRM —
otherwise REVIEW_REQUIRED.

## Spawn model
Default implementor (z-ai/glm-5.3-flash via OpenRouter, free tier); context
ceiling ≤ 50% of the model window at spawn (flash-class window ≥ 128k;
one-file YAML edit will not approach it).
