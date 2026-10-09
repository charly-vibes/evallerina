# Project Context

## Purpose

**evallerina** is the consumer eval battery for the **dulce-de-leche tool
family** — the charly-vibes CLI suite built on genesis-vibes. It answers one
question empirically: **do LLM agents read, parse, and act on the suite's AIX
output channels?** — JSON envelopes (`ok`, `warnings`, `hints`), self-healing
suggestions, managed blocks in `AGENTS.md`, state machines, `llms.txt`
discovery.

Every eval must produce an **actionable signal**: which output channel fails
for which model tier — never a single aggregate score. Roadmap and
issue-level detail live in beads (`bd ready`, `bd list`); this file documents
only agent-relevant context.

## Tech Stack

- **Rust** — harness crate (`src/`), depending on genesis-vibes (`Fixture`,
  `evals` module) per genesis `docs/how-to/evals.md`
- **just** — recipe entry points (`justfile`: `tier0`, `tier1`, `tier2`,
  `smoke`, `ci`)
- **GitHub Actions** — tier-0 gate on push, tier-1 nightly replay, tier-2
  scheduled rotation (planned: evallerina-5ap / evallerina-aay)
- **beads + openspec + wai** — issue tracking, change proposals, research
  capture

## Project Conventions

### Code Style

- `cargo fmt --check` and `cargo clippy -- -D warnings` gate every push
  (tier-0)
- Every new source file carries a **Purpose / Responsibilities / Rationale**
  header (Must gate on evallerina-e10)
- `.editorconfig` and `_typos.toml` standardize formatting and prose

### Architecture Patterns

- **Tier ladder** (normative: genesis `evals-guidelines` spec) — tier 0 static
  checks (every push, no model) / tier 1 scripted replay of recorded
  transcripts (nightly, no model) / tier 2 live model runs (scheduled,
  never gates a push)
- **Sandboxed fixtures** — fresh temp dir, tool binaries pre-provisioned,
  isolated `HOME`, network denied unless the scenario opts in
- **Process-boundary scoring** — assert only on exit codes, captured
  stdout/stderr envelopes, and filesystem state; free-form agent text is
  never scored
- **Synthetic repo archetypes** — deterministic fixture repos with injected
  faults, replayed via the tier-0/1 recorded-trajectory harness
  (evallerina-sqq: 6 archetypes)

### Testing Strategy

- Tier-0 (`just ci`) gates every push: fmt + clippy + test
- Tier-1 replay scenarios assert envelope outcomes over recorded AgentStep
  transcripts; Must gate on the harness is ≥1 smoke scenario replaying a
  recorded wai trajectory (wai = first target, most adopted)
- Tier-2 live runs require `OPENROUTER_API_KEY`, record raw model ids
  verbatim, n ≥ 3 reps per scenario × model cell, 429 → one retry then
  `rate_limited`
- Every failure classified via the fault taxonomy
  (`ERR_ENVELOPE_HINT_BLINDNESS`, `ERR_DOC_DRIFT_BLINDNESS`,
  `ERR_ACTION_FORMAT_VIOLATION`, …) and routed — tool faults become tickets
  in the tool's repo; agent faults reproducible across ≥ 3 models point at
  the output channel (AIX work in genesis)

### Git Workflow

- Single `main` branch, no branches yet
- Conventional-style commit subjects (observed: `chore:`, `beads:`,
  `chore(beads):`)
- Issue-level provenance in beads: `base_commit` + expected `files` per
  ticket (pattern set by the evallerina-sqq remediation)
- Commit beads changes only — `issues.jsonl` is the source of truth (embedded
  + `no-db:true`); transient gate locks (`*.gate.lock`) are gitignored

## Domain Context

- The suite being evaluated: **dulce-de-leche** (wai, dont, ah, vampiro,
  pretender, …) — wai is the first eval target, most adopted
- AIX = agent-experience artifacts: the A/B ablation family measures the
  delta of AIX artifacts present vs absent — the measured value of the
  documentation investment
- Genesis provides the normative specs this repo consumes: `evals-guidelines`
  spec, evals how-to, eval-report contract (see README Related section)
- Report rows follow the genesis `eval-report` JSON contract (`report_version`)

## Important Constraints

- **No model in gate paths**: tier-0/1 never call a model; tier-2 never gates
  a push
- **No secrets in traces**: no credentials, personal data, or sensitive
  prompt/trace content in committed scenarios or reports
- **Network denied** in fixture sandboxes unless the scenario explicitly opts
  in
- New harness code follows genesis pattern (Ticket → base_commit → files →
  Must gate → implement → validate → commit)
- Free-tier entry: OpenRouter `:free` ids are the default candidates of the
  model registry

## External Dependencies

- **OpenRouter API** — tier-2 live model runs (`OPENROUTER_API_KEY` env var)
- **genesis-vibes** — harness crate dependency (Fixture + evals)
- **GitHub** / gh CLI — workflows, authenticated locally
- Genesis repo docs (spec source of truth)
