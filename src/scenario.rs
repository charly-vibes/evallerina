//! Purpose: Scenario definitions for the evallerina battery.
//! Responsibilities: assemble tier-1 replay scenarios — fixture setup,
//! agent prompt, and deterministic checks — starting with the smoke
//! Must gate of evallerina-e10 (one recorded wai trajectory proving the
//! harness end-to-end).
//! Rationale: scenarios follow the eval formula (fixture + prompt +
//! deterministic checks + error code) and carry provenance in the
//! recorded trajectory file. New scenario families (hint adherence,
//! doc-drift blindness, managed-block audits) land here as separate
//! tickets with their own recorded fixtures.

use crate::envelope::{
    agent_followed_hint_loose, error_envelope_with_remediation_hint, ok_envelope_loose,
};
use genesis::evals::{Scenario, agent_executed_all};

/// Directory holding recorded trajectories, relative to the crate root.
pub const SCENARIOS_DIR: &str = "scenarios";

/// The smoke scenario (evallerina-e10 Must gate).
///
/// Replay shape, recorded live against wai 2026.10.5
/// (`scenarios/wai-hint-adherence-smoke.json`):
/// 1. `wai status --json` in an uninitialized directory — exits 1 with
///    error envelope E000 whose remediation suggests `wai doctor`
/// 2. the agent follows the hint: `wai doctor --json` — still exits 1
///    (doctor cannot repair an uninitialized project; the channel is
///    honest, the recovery path is just two hops)
/// 3. the agent resolves the root cause: `wai init --json` — exits 0
///    with `ok:true`
///
/// Checks, one fault each:
/// - `status-error-envelope-carries-remediation-hint` (tool fault if broken)
/// - `agent-followed-hint` (agent fault `ERR_ENVELOPE_HINT_BLINDNESS`; prefix-tolerant — see
///   [`crate::envelope::agent_followed_hint_loose`] for why genesis's exact-match
///   helper flags flag-bearing replays)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `init-recovery-ok` (tool fault if broken)
pub fn smoke_scenario() -> Scenario {
    Scenario::new(
        "wai-hint-adherence-smoke",
        "Run diagnostics on this directory and resolve the problem it reports.",
    )
    .check(
        "status-error-envelope-carries-remediation-hint",
        error_envelope_with_remediation_hint(0, "wai doctor"),
    )
    .check(
        "agent-followed-hint",
        agent_followed_hint_loose(1, "wai doctor"),
    )
    .check("agent-executed-all", agent_executed_all())
    .check("init-recovery-ok", ok_envelope_loose(2))
}
