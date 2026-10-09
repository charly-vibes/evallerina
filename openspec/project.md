# Project Context

## Purpose
evallerina is the consumer eval battery for the dulce-de-leche (ddl) tool family. It runs tiered evaluations of agentic CLI tools against AIX artifacts (llms.txt, managed AGENTS.md, .genesis/tools.toml) to measure where output-channel faults come from — AIX channel work vs agent faults — with per-capability breakdowns and no single aggregate score (per cli-agent-evals-prompt.md anti-goals). A/B ablation (evallerina-rl1) quantifies the value of the AIX investment via score deltas.

## Tech Stack
- Rust (harness crate, genesis-vibes Fixture + evals module per genesis docs/how-to/evals.md)
- just (task runner): tier0 (fmt/clippy/test), tier1 (recorded replay), tier2 (live OpenRouter)
- beads (embedded Dolt, no-db:true — issues.jsonl is the source of truth)
- wai workspace + OpenSpec (specs/, changes/)
- GHA workflows (planned): tier-0 push gate, tier-1 nightly replay, tier-2 scheduled rotation

## Project Conventions

### Code Style
- New files carry Purpose/Responsibilities/Rationale headers (per evallerina-e10 Must gate)
- cargo fmt --check + clippy -D warnings must pass (tier0 gates every push)

### Architecture Patterns
- Tier ladder per genesis evals-guidelines: tier-0 static (push, no model), tier-1 replay of recorded AgentStep transcripts (nightly, no model/network), tier-2 live model runs via OpenRouter :free (scheduled, never gates a push)
- Tier-2 runner (1vw) uses an action protocol; never gates a push
- Scenario families: hint adherence via contrived-failure injection (ey7), doc-drift blindness with stale AGENTS.md bait (7y1), 6 synthetic repo archetypes for cross-tool workflows (sqq)
- Report aggregation: per-output-channel x check-code x model-id matrix; fault routing (tool faults → tool repo tickets; agent faults across ≥3 model ids → AIX channel work in genesis). rate_limited and absent cells are never dropped or silently substituted

### Testing Strategy
- Tier-0/1 first: Scenario::run over recorded wai trajectories; envelope-assertion helpers; ErrorTaxonomy classification (e10)
- Every eval run twice per evals.md Step 5: full (AIX artifacts) vs ablated (raw binaries); delta over ≥3 replay runs per arm is the measured value
- KPI assertions per the Qwen doc: context-recovery time, test-selection

### Git Workflow
- trunk-based on main; beads issues with explicit BLOCKS dependency graph; work items carry base_commit metadata
- Direct commit conventions: chore(beads)/chore + concise imperative subject

## Domain Context
- Genesis = design system repo; evals-guidelines defines the tier ladder and fault routing; evals.md defines the run protocol (Steps 5, 7)
- wai is the first target tool family (most adopted); ah/dont/genesis/vampiro also in scope for cross-tool suites
- Drifted fixtures intentionally include stale manifests, registry drift, stale AGENTS.md bait (overlaps sqq archetype 6 / 7y1)

## Important Constraints
- Tier-2 requires OPENROUTER_API_KEY; live runs never gate a push
- Anti-goal: no single aggregate score; missing/absent cells must be explicit
- No interactive hooks for DevOps/headless archetype (CI-only)

## External Dependencies
- OpenRouter (:free tier models, default deepseek/deepseek-v4-flash:free) for tier-2 live runs
- genesis-vibes crate (Fixture + evals module) for the harness
- recorded wai AgentStep transcripts for tier-1 replay
