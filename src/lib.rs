//! Purpose: Library core of the evallerina harness — the consumer eval
//! battery for the dulce-de-leche tool family (wai first, most adopted).
//! Responsibilities: parse the family's envelope channels (which drift
//! from genesis's generic `hints` shape), load recorded agent
//! trajectories from `scenarios/`, and assemble tier-1 replay scenarios
//! with deterministic checks from `genesis::evals`.
//! Rationale: process-boundary scoring per the genesis evals-guidelines
//! spec — the harness only ever asserts on captured stdout/stderr, exit
//! codes, and fixture state; free-form agent text is never scored. The
//! dulce-specific envelope readers live here because the reusable
//! genesis helpers assume a top-level `hints` array, while real tool
//! output (wai 2026.10.5) carries remediation under `data.remediation`.

pub mod ablation;
pub mod envelope;
pub mod live;
pub mod recorded;
pub mod registry;
pub mod report;
pub mod scenario;
