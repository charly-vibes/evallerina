#+TITLE: Verify: evallerina-2bz — Route: filing path (dedup search + bd create in target repo)
#+DATE: [2026-10-19]
#+FILETAGS: :verify:

Read-only verification. Implementor's report missing (budget kill); verified
directly from repo reality.

* 1. Commits (git log / git show --stat)

Four task commits present, atop the prior l8w boundary:

- ~9c41b26~ feat(routing): file path — token dedup, open match => existing,
  zero bd create (task 2.1). ~--stat~: =src/routing.rs | 145 +…+=,
  =tests/routing.rs | 128 +…+=.
- ~6b6ed88~ test(routing): only a CLOSED issue holds the token => re-filed,
  not suppressed (task 2.2). =tests/routing.rs | 26 +=.
- ~8e78ec9~ test(routing): --file invokes bd create in the target repo cwd,
  captures id + tracked export (tasks 2.3–2.5). =tests/routing.rs | 61 +=.
- ~e3a8234~ test(routing): bd create failure => skipped/bd_failure, never
  fatal (task 2.6). =src/routing.rs | 3 +-~, =tests/routing.rs | 149 …=.

All four confirmed via ~git show --stat~. ✅

* 2. Isolation (tests/routing.rs, src/routing.rs)

- Fixture: =Sandbox::fresh= creates dirs under =std::env::temp_dir()= with
  pid+nanos uniqueness (tests/routing.rs:39–50); =Drop= removes them
  (tests/routing.rs:76–80). ~bd init~ runs only inside sandboxes
  (=with_beads=, line 55).
- Filing tests all pass sandbox paths: every =failable_row(...)= call site
  (lines 369, 408, 427, 480, 505, 517, 518, 536) uses =&repo.path= or
  =&not_a_beads_repo.path= — never a family-repo path.
- The =../wai= / =../genesis= strings appearing in tests are inert: in
  =repo_map()= (line 167) they feed the dry-run =propose= mapping (pure
  in-memory), and =resolve_repo("../wai", root)= (lines 282–297) is a pure
  string join in src/routing.rs:267–273 — no filesystem access. ✅
- =.beads= references in tests are all =<sandbox>/.beads/issues.jsonl=
  (e.g. line 485 =repo.path.join(".beads/issues.jsonl")=). No test touches
  evallerina's own .beads or family repos. ✅
- ~BdSpy~ records every =(cwd, args)= invocation, enabling the zero-create
  and cwd assertions.
- bd search pinning: the only =search= invocation in src/ is
  src/routing.rs:363 — =run_bd(&cwd, &["search", &row.token, "--status",
  "open", "--json")=. The fallback (line 368) uses =bd list --status open
  --json= + grep — same filter. Tests assert the pin explicitly
  (tests/routing.rs:384–385: *"dedup search must pass --status open"*). No
  =bd search= without =--status open= anywhere. ✅

* 3. Fresh runs (actual output)

** ~cargo test --test routing~ (exit 0)

#+BEGIN_EXAMPLE
running 18 tests
test absolute_repo_passes_through_for_worktrees ... ok
test agent_faults_are_never_routed ... ok
test below_threshold_is_no_proposal ... ok
test every_proposal_row_carries_the_full_schema ... ok
test jsonl_writer_emits_one_object_per_line ... ok
test relative_repo_resolves_against_evallerina_root ... ok
test repo_keys_on_recorded_tool_not_row_tool ... ok
test unknown_scenario_is_a_loud_error ... ok
test routing_toml_parses_tools_to_repos ... ok
test routing_toml_missing_repo_is_loud ... ok
test threshold_tool_fault_becomes_dry_run_proposal ... ok
test unmapped_tool_is_skipped_not_fatal ... ok
test bd_search_spawns_may_fail_too_but_dedup_falls_back ... ok
test open_issue_with_token_dedups_to_existing_without_creating ... ok
test filing_refreshes_tracked_export_in_target_repo ... ok
test filing_invokes_bd_create_in_target_repo_cwd_and_captures_the_id ... ok
test bd_failure_in_target_repo_is_skipped_never_fatal ... ok
test closed_issue_does_not_block_refiling_a_regressed_fault ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.80s
EXIT=0
#+END_EXAMPLE

18 passed as expected. ✅

** ~just tier0~ (exit 0)

=cargo fmt --check= clean; =cargo clippy -- -D warnings= clean (=Finished
dev profile=); full =cargo test=: lib 3 passed, ablation 10 passed,
evals_replay 16 passed, live_protocol 15 passed, report_aggregation 17
passed, routing 18 passed, doc-tests 0. =EXIT=0=. ✅

* 4. Spec conformance vs acceptance criteria

| Criterion (ticket) | Test | Evidence |
|---+---+---|
| dedup: open match → ~existing~, zero bd create | ~open_issue_with_token_dedups_to_existing_without_creating~ | spy asserts no =create= calls; =bd_issue_id= stays null for existing |
| closed-ticket-refile: closed-only match → filed, never suppressed | ~closed_issue_does_not_block_refiling_a_regressed_fault~ | pins =--status open= so closed match doesn't dedup; fresh =bd_issue_id= |
| file-capture: bd create runs in target repo cwd, issue id captured | ~filing_invokes_bd_create_in_target_repo_cwd_and_captures_the_id~ | spy asserts cwd == target repo; ~--json~ id parsed; =bd_issue_id= carries target prefix; tracked-export refreshed (=.beads/issues.jsonl= contains new id) — ~filing_refreshes_tracked_export_in_target_repo~ |
| bd-failure: non-beads dir → ~skipped(bd_failure)~, report succeeds | ~bd_failure_in_target_repo_is_skipped_never_fatal~ | real non-beads temp dir; sibling healthy repo still files |

All four acceptance-criteria behaviors have dedicated tests, all green. ✅

* 5. Tracker state

~bd show evallerina-2bz~: =◐ evallerina-2bz · Route: filing path (dedup
search + bd create in target repo)  [P1 · IN_PROGRESS]= — in_progress, not
closed. ✅ (Note: lease shows expired, heartbeat 43 min stale — consistent
with the budget kill mid-run, harmless for verification.)

* Verdict

CONFIRM — all four commits verified via =git show --stat=; tests are fully
temp-dir isolated with no family-repo writes and =--status open= pinned on
every dedup search; fresh =cargo test --test routing= = 18 passed (exit 0);
fresh =just tier0= exit 0; ticket remains in_progress with dedicated green
tests for dedup, closed-refile, file-capture, and bd_failure.

Minor non-blocking observations (for the implementor's close-out, not this
verification):
- Ticket lease expired; =bd update evallerina-2bz= heartbeat needed before
  claiming continuation work.
- Tracked-export invariant in evallerina's own repo unchanged by tests
  (verified: no test writes to repo-local =.beads=).
