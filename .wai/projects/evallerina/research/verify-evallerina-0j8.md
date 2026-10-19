# VERIFY — evallerina-0j8 (real AIX artifact generation in the ablation full arm)

Read-only verification, no implementor report available (implementor wedged and
was killed after committing all 4 steps). Verified purely from git history,
current source, fresh test runs, and the tracker.

Date: 2026-10-19 · Verifier: read-only agent session

## 1. Commit confirmation (git show --stat + full diffs read)

| Commit | Claim | Scope | Verdict |
|---|---|---|---|
| `b03f469` | real managed AGENTS.md generation in full arm | `src/ablation.rs` (+161), `tests/ablation.rs` (+80) | ✅ CONFIRMED |
| `20863e3` | real llms.txt via genesis `generate_llms_txt` | `src/ablation.rs` (+37), `tests/ablation.rs` (+36) | ✅ CONFIRMED |
| `f628b68` | hermetic fallback without opt-in | `tests/ablation.rs` only (+35) | ✅ CONFIRMED |
| `395948d` | loud generation failures + refuses scenario-owned paths | `tests/ablation.rs` only (+104) | ✅ CONFIRMED |

Key evidence from diffs:

- **Step 1 (`b03f469`, src/ablation.rs:79-146, src/ablation.rs:261-287):**
  `AixSource { Static(&AixArtifacts), Real(ToolBinaries) }` enum added;
  `AixArtifacts::for_fixture()` runs the provisioned `wai init --json` in a
  dedicated `generation_fixture_dir()` (fresh pid+nanos temp dir — never the
  scenario's trial fixture), asserts exit 0, then reads back the real
  `AGENTS.md` and registered `.genesis/tools.toml`. Any failure (spawn error,
  nonzero exit, unreadable file) is a `Result<_, String>` loud error. Static
  remains the default.
- **Step 2 (`20863e3`, src/ablation.rs:107-146):** the step-1 stub
  (`llms_txt: String::new()` with "wires up next step" comment) is replaced by
  `llms_txt_for_fixture()`: `genesis::discovery::read_manifest` → map to
  `genesis::aix::ModuleEntry` (sorted for order stability) →
  `genesis::aix::generate_llms_txt` with the fixture's project name. Real
  genesis output, not hand-rolled text.
- **Step 3 (`f628b68`, tests/ablation.rs:225-261):**
  `static_source_provisions_hermetic_contents_unchanged` pins that
  `AixSource::Static` provisions byte-for-byte today's contents (full arm
  passes a content-equality check; ablated arm fails 3/3).
- **Step 4 (`395948d`, tests/ablation.rs:263-362):** fake shell-script `wai`
  binaries pin the contract: exit-3 → loud error naming "wai init" through
  both `for_fixture` AND `run_ablation` (no silent placeholder fallback);
  exit-0-with-no-AGENTS.md → loud error naming the file; scenario-owned
  `.genesis/tools.toml` (the corrupt-config fault injection) → refused in real
  mode before any generation runs (collision check covers both sources, since
  it iterates `ARTIFACT_PATHS` before generation — src/ablation.rs:262-275).

## 2. Anti-goals checked against current source

- **No placeholder content in the real path** — ✅. `grep placeholder
  src/ablation.rs` hits only doc-comments; the placeholder strings
  ("managed guidance", "wai status --json") appear only in *negative*
  assertions (`!generated.managed_agents_md.contains("managed guidance")`,
  tests/ablation.rs:179,222). Placeholder text lives solely in the opt-out
  `AixSource::Static` path, as designed. Independently confirmed real wai
  output shape: ran `wai init --json` in a scratch temp dir — produced the
  real `<!-- WAI:START -->` managed AGENTS.md and a `[tools.wai]` manifest.
- **No silent fallback on tool failure** — ✅. Real-mode generation is
  process-boundary with explicit error paths for spawn failure, nonzero exit
  (stderr surfaced), and missing produced files; step-4 tests prove the error
  propagates out of `run_ablation` unchanged.
- **No collision with scenario-owned fixture files (`.genesis/tools.toml`)** —
  ✅. Collision loop runs over `ARTIFACT_PATHS` *before* generation in both
  modes (src/ablation.rs:261-275) and generation happens in a dedicated temp
  generation fixture, never the trial fixture. Pinned by
  `real_mode_refuses_scenario_owned_paths_before_generation` and
  `artifact_path_collision_is_a_loud_error`, both passing.
- **Routing/report code untouched** — ✅. All four commits touch only
  `src/ablation.rs` + `tests/ablation.rs`; no routing.rs / report files in any
  diff. tier0's routing (18 tests) and report_aggregation (23 tests) suites
  pass unchanged.

## 3. Fresh runs (actual output)

`cargo test --test ablation` (extract):

```
running 16 tests
test artifact_fixture_files_map_the_three_paths ... ok
test artifact_path_collision_is_a_loud_error ... ok
test real_mode_refuses_scenario_owned_paths_before_generation ... ok
test reps_below_three_is_an_error ... ok
test missing_produced_file_is_a_loud_error ... ok
test tool_generation_failure_is_a_loud_error ... ok
test real_generation_llms_txt_is_genesis_output_for_the_tool_set ... ok
test real_generation_writes_managed_agents_md_with_wai_provenance ... ok
test plain_rotation_rows_default_to_full_arm ... ok
test absent_cells_surfaced_and_excluded_from_delta_denominator ... ok
test rows_carry_arm_and_report_version_stays_1 ... ok
test mixed_results_produce_fractional_delta ... ok
test ablation_report_serializes_with_per_arm_detail ... ok
test delta_computed_over_three_runs_per_arm ... ok
test static_source_provisions_hermetic_contents_unchanged ... ok
test delta_undefined_when_an_arm_cannot_run ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.13s
```

16 passed, as expected. Note: the two `real_generation_*` tests did **not**
hermetically skip — `wai` is provisioned at `~/.cargo/bin/wai`, so they ran
real generation for real (consistent with the 6.13 s wall time; the hermetic
skip would have been near-instant).

`just tier0` → **EXIT=0**:

```
cargo fmt --check
cargo clippy -- -D warnings   # Finished `dev` profile
cargo test
  unittests src/lib.rs        3 passed
  unittests src/main.rs       0 tests
  tests/ablation.rs          16 passed
  tests/evals_replay.rs      16 passed
  tests/live_protocol.rs     15 passed
  tests/report_aggregation.rs 23 passed
  tests/routing.rs           18 passed
  Doc-tests evallerina        0 tests
```

## 4. Tracker

`bd show evallerina-0j8` →

```
◐ evallerina-0j8 · Wire real artifact contents into ablation harness  [P2 · IN_PROGRESS]
Started: 2026-10-19 · Lease: expires expired (heartbeat 41 mins ago)
```

✅ IN_PROGRESS as claimed. Caveat: the heartbeat lease is expired (implementor
was killed mid-session) — consistent with the wedged-agent narrative, but the
ticket should either be resumed or closed by whoever picks up the work.

## Read paths

- `src/ablation.rs` — AixSource, ToolBinaries, for_fixture, generation_fixture_dir,
  llms_txt_for_fixture, collision-before-generation ordering
- `tests/ablation.rs` — 6 new tests across steps 1/2/3/4
- Git: `git show b03f469 | 20863e3 | f628b68 | 395948d` (+ `--stat`)
- `/tmp/verify-0j8-gen/AGENTS.md`, `.genesis/tools.toml` — independent probe of
  real wai output shape

## Verdict

**CONFIRM** — all 4 commits exist, match their claims, and are scoped exactly to
`src/ablation.rs` + `tests/ablation.rs`; fresh `cargo test --test ablation` is
16/16 (with real tool generation actually executing, not skipping) and
`just tier0` exits 0; no placeholder content on the real path, tool failures
are loud, scenario-owned paths are refused before generation, and routing/
report code is untouched. Remaining follow-up (not a verification failure):
ticket `evallerina-0j8` is still IN_PROGRESS and needs to be closed once
someone reviews this artifact and re-exports `.beads/issues.jsonl`.
