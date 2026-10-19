# Brief: evallerina-0j8 — real artifact contents in the ablation full arm

Ticket: evallerina-0j8 (P2, complexity:l) — wire real AIX artifact contents
into the ablation harness (src/ablation.rs, src/live.rs ArmContext).

## Why
The ablation delta only measures the actual documentation investment if the
full arm provisions what the tools really generate. Today AixArtifacts
carries &'static placeholder strings. Design decision (recorded in wai
research): let the provisioned tools generate their artifacts inside the
full arm's fixture at runtime — the harness already pre-provisions tool
binaries; `wai init --json` in the fixture yields the real managed
AGENTS.md block; genesis's `generate_llms_txt` (genesis-vibes crate) yields
the real llms.txt for the fixture tool set.

## Desired outcome
`AixArtifacts::for_fixture(fixture_dir, tool_binaries)` (name flexible):
generates real contents by invoking the provisioned tools in the fixture
(process-boundary: assert on exit codes + produced files, never internals).
Static `&'static str` fields remain for hermetic tests; production entry
returns owned contents. `run_ablation` gains a generation step for the full
arm. Real-tool generation must be opt-in (env or parameter) so CI without
tool binaries still runs hermetically — fallback to current behavior.

## Exact files
- `src/ablation.rs` (extend; keep header current)
- `src/live.rs` only if ArmContext plumbing requires it
- `tests/ablation.rs` (extend)

## Red/green sequence
1. RED: full arm with real-generation enabled writes a managed AGENTS.md
   containing wai's provenance footer (managed block format) — assert
   contains the block markers from genesis-vibes managed_block, not the
   placeholder text. GREEN.
2. RED: llms.txt content is genesis `generate_llms_txt` output (contains
   module section markers). GREEN.
3. RED: without tool binaries / opt-out → hermetic fallback, behavior
   unchanged from today. GREEN.
4. RED: tool generation failure → loud error (corrupt-config scenario owns
   .genesis/tools.toml — do not collide; see artifact_path_collision test).
   GREEN.

## Anti-goals
- No placeholder content in the real path; no silent fallback on failure
- No collision with scenario-owned fixture files
- Do not modify routing/report code (different concern)

## Commands
- `cargo test --test ablation` per step; `just tier0` green at end

## Follow-up threshold
If genesis-vibes API doesn't expose what's needed (>1h), STOP and record.

## Spawn model
Default implementor; context ceiling ≤ 50%.
