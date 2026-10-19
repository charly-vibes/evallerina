#+TITLE: Verify: evallerina-wdx — 'evallerina ablate' CLI subcommand
#+DATE: [2026-10-19]
#+PROPERTY: mode read-only verification

* Claim under test

Implementor claims commits =186cdad= (step 1, CLI surface red→green) and
=574a31a= (step 2, wire =run_ablation= red→green), tests green.

* Verification results

** 1. Commits exist (git show --stat)

Both commits present with the claimed content:

: commit 186cdad — "evallerina-wdx step 1: 'evallerina ablate' CLI surface (red→green)"
:   src/main.rs         | 73 ++++ | tests/ablate_cli.rs | 44 ++++ | 2 files, +117
:
: commit 574a31a — "evallerina-wdx step 2: wire ablate to run_ablation (red→green)"
:   src/main.rs         | +++/--- | tests/ablate_cli.rs | ++++ | 2 files, +220/-5

Note: =src/ablation.rs= and =tests/ablation.rs= appear in *neither* commit's
diffstat — step 2 touched only =src/main.rs= and =tests/ablate_cli.rs=
(see §3).

** 2. Fresh runs

=cargo test --test ablation= → *16 passed; 0 failed*, finished in 6.13s:

: test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

=cargo test --test ablate_cli= (bonus, new CLI tests) → *4 passed; 0 failed*:

: test ablate_unwritable_out_path_fails_loudly ... ok
: test ablate_dry_run_writes_report_files_in_temp_sandbox ... ok
: test ablate_unknown_scenario_fails_loudly ... ok
: test result: ok. 4 passed

=just tier0= → exit 0 (last tier: =test result: ok. 18 passed; 0 failed=).

=cargo run -- ablate --help= → exit 0, full usage text (abridged):

: Usage: evallerina ablate [OPTIONS] --out <OUT> <SCENARIO>
:
: Arguments:
:   <SCENARIO>  Scenario name (a live battery scenario; unknown names are a loud error)
:
: Options:
:       --reps <REPS>                ... [default: 3]
:       --model <MODEL>              ...
:       --dry-run                    Dry-run: every cell is recorded absent — no model calls, no network
:       --real-artifacts             Real artifact generation (opt-in; evallerina-0j8)
:       --budget-mins <BUDGET_MINS>  ... [default: 20]
:       --out <OUT>                  Path the ArmReport JSONL rows are written to ...

(First help invocation had EXIT captured after =tail= — reran as
=cargo run --quiet -- ablate --help= directly: =help exit: 0=.)

** 3. Anti-goals

- *No src/ablation.rs modification*: =git show 574a31a -- src/ablation.rs= and
  the 186cdad stat query both return empty diffs for that path. ✔
- *No live model calls / network in new tests*: read =tests/ablate_cli.rs= —
  the dry-run test asserts success with
  =.env_remove("OPENROUTER_API_KEY")= and message "dry-run succeeds
  without a key or network". Grep for OPENROUTER in the new tests shows
  only =env_remove= calls (lines 52/107/132) — tests *strip* the key and
  assert success, proving the path needs neither key nor network. =src/main.rs=
  line 312 confirms the transport "refuses to start without
  OPENROUTER_API_KEY", so a passing keyless dry-run is genuine evidence
  of no model calls. ✔
- *Loud error on unknown scenario*: ran
  =cargo run -- ablate unknown-scenario --out /tmp/x.jsonl= — exit 1,
  stderr names the scenario and lists live ones:

: unknown scenario 'unknown-scenario' — live scenarios: wai-hint-adherence-smoke, ...

  Also covered by test =ablate_unknown_scenario_fails_loudly=. ✔

** 4. bd show evallerina-wdx

: ◐ evallerina-wdx · 'evallerina ablate' CLI subcommand  [P3 · IN_PROGRESS]

Status =IN_PROGRESS= as claimed. ✔

* Verdict

CONFIRM

All four verification axes check out from fresh runs: commits match
claims, 16/16 ablation tests + 4/4 CLI tests green, tier0 exit 0,
=ablate --help= exits 0 with usage, =src/ablation.rs= untouched,
tests are hermetic (key stripped, success asserted), unknown scenario
fails loudly with exit 1, ticket is in_progress.
