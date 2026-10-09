# evallerina — eval battery for the dulce-de-leche tool family
# Tier ladder per genesis evals-guidelines: 0 static (push) / 1 replay (nightly) / 2 live (scheduled)

default:
    @just --list

# Tier 0 — static checks, no model (gates every push)
tier0:
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo test

# Tier 1 — scripted replay of recorded AgentStep transcripts, no model, no network
tier1:
    cargo test --test evals_replay -- --nocapture

# Tier 2 — live rotation (requires OPENROUTER_API_KEY); never gates a push.
# One JSON event per cell on stdout (trial row or absent record).
tier2 model="openrouter/free" scenario="" budget="20":
    #!/usr/bin/env bash
    set -euo pipefail
    args=(live --model "{{model}}" --budget-mins "{{budget}}")
    if [ -n "{{scenario}}" ]; then args+=("{{scenario}}"); fi
    cargo run --release -- "${args[@]}"

# Build + test the harness crate
ci: tier0

# Fixture sandbox smoke test: provision binaries, run one replay scenario
smoke:
    cargo run --release -- smoke
