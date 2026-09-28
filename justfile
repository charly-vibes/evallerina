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

# Tier 2 — live model runs (requires OPENROUTER_API_KEY); never gates a push
tier2 model="deepseek/deepseek-v4-flash:free" scenario="":
    OPENROUTER_MODEL="{{model}}" cargo run --release -- live {{scenario}}

# Build + test the harness crate
ci: tier0

# Fixture sandbox smoke test: provision binaries, run one replay scenario
smoke:
    cargo run --release -- smoke
