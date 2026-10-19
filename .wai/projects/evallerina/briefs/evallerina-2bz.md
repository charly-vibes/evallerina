# Brief: evallerina-2bz — filing path (dedup + bd create)

Ticket: evallerina-2bz (P1, afk, complexity:m) — Phase 2 of openspec
`add-fault-ticket-routing`, tasks 2.1–2.6. Extends src/routing.rs landed by
evallerina-l8w (read it first; its header documents current state).

## Why
Filing touches a foreign process boundary (`bd` in other repos) and needs
hard isolation discipline — a subagent with a tight brief avoids polluting
the lead context with process-detail back-and-forth.

## Desired outcome
Dry-run proposals can be *filed*: dedup search in the target repo, then
`bd create` there, with every failure mode routed per spec.

## Exact files
- `src/routing.rs` (extend; keep the file header current)
- `tests/routing.rs` (extend)

## Red/green sequence (tasks 2.1–2.6)
1. RED 2.1: dedup — temp dir, `bd init`, seed an OPEN issue whose body
   contains token `evallerina:<scenario>:tool_fault` → decision `existing`,
   zero `bd create` calls. GREEN.
2. RED 2.2: only a CLOSED issue holds the token → proposed + `filed`.
   GREEN. (`bd search` includes closed by default — always pass
   `--status open`; that is why 2.1/2.2 differ.)
3. RED 2.3: `--file`-authorized path invokes `bd create` in the target repo
   cwd (temp repo with `bd init`), captures exit code + issue id.
   GREEN. `bd export -o .beads/issues.jsonl` afterwards if the target repo
   has one (tracked-export invariant) — test that too.
4. RED 2.6: `bd create` fails (non-beads dir) → decision `skipped`, reason
   `bd_failure`, no error propagated. GREEN.

## Isolation rules (anti-goals — hard)
- ALL bd operations in tests run inside `std::env::temp_dir()` fixtures
  created per-test. NEVER touch the evallerina/wai/genesis/espectacular/dont
  repo trackers from tests.
- If spawning `bd` in-process is flaky in tests, isolate via a helper that
  sets the subprocess cwd to the temp repo; assert on exit codes + parsed
  ids (process-boundary principle).
- No `bd search` invocation without `--status open`.

## Commands
- `cargo test --test routing` per step; `just tier0` green at end.

## Out of scope
- CLI flag/report wiring (evallerina-o3j), no edits to main.rs/report.rs.

## Follow-up threshold
If `bd create` output parsing proves fragile across bd versions, STOP and
record findings; do not build a parser for every format.

## Spawn model
Model: sonnet or default implementor. Context ceiling: ≤ 50% at spawn.
