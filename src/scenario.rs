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
    agent_followed_hint_loose, envelope_field_is, error_envelope_with_hint_containing,
    error_envelope_with_remediation_hint, ok_envelope_loose, stderr_did_you_mean,
};
use genesis::evals::{Scenario, agent_executed_all};

/// Directory holding recorded trajectories, relative to the crate root.
pub const SCENARIOS_DIR: &str = "scenarios";

/// Subdirectory holding the hint-adherence family (evallerina-ey7).
pub const HINT_ADHERENCE_DIR: &str = "scenarios/hint-adherence";

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

/// The typo scenario (evallerina-ey7 hint-adherence family).
///
/// Contrived failure: unknown subcommand. Recorded live against wai
/// 2026.10.5 (`scenarios/hint-adherence/wai-hint-adherence-typo.json`):
/// 1. `wai statuss --json` — exits 2 with the DidYouMean hint on stderr
///    text only (`Did you mean 'status'?`). No JSON envelope: wai's
///    clap-level typo channel predates the envelope, so this scenario
///    exercises the stderr-text hint channel, unlike the smoke's.
/// 2. the agent follows the suggestion: `wai status --json` — exits 1
///    with error envelope E000 (uninitialized directory)
/// 3. the agent resolves the root cause: `wai init --json` — ok:true
///
/// Checks, one fault each:
/// - `typo-stderr-carries-did-you-mean` (tool fault if the DidYouMean
///   hint is absent from stderr — the channel is the tool's promise)
/// - `agent-followed-did-you-mean` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `init-recovery-ok` (tool fault if broken)
pub fn wai_typo_scenario() -> Scenario {
    Scenario::new("wai-hint-adherence-typo", "Run status on this directory.")
        .check(
            "typo-stderr-carries-did-you-mean",
            stderr_did_you_mean(0, "status"),
        )
        .check(
            "agent-followed-did-you-mean",
            agent_followed_hint_loose(1, "wai status"),
        )
        .check("agent-executed-all", agent_executed_all())
        .check("init-recovery-ok", ok_envelope_loose(2))
}

/// The corrupt-config scenario (evallerina-ey7 hint-adherence family).
///
/// Contrived failure: malformed config injection. Recorded live against
/// wai 2026.10.5 (`scenarios/hint-adherence/wai-hint-adherence-corrupt-config.json`);
/// the fixture materializes the same malformed `.wai/config.toml`:
/// 1. `wai status --json` — exits 1 with error envelope E000 (config
///    parse error) whose remediation suggests `wai doctor`
/// 2. the agent follows the hint: `wai doctor --json` — exits 1 with an
///    ok:true `doctor` envelope whose hints include `Fix the syntax in
///    .wai/config.toml` (doctor diagnoses; it cannot fix TOML syntax)
/// 3. the agent hand-repairs the config (printf of a minimal valid one)
/// 4. `wai status --json` — ok:true
///
/// Checks, one fault each:
/// - `config-error-envelope-carries-remediation-hint` (tool fault)
/// - `agent-followed-doctor-hint` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `status-recovery-ok` (tool fault if broken)
pub fn wai_corrupt_config_scenario() -> Scenario {
    Scenario::new(
        "wai-hint-adherence-corrupt-config",
        "Run status on this directory and resolve the problem it reports.",
    )
    .fixture_file(".wai/config.toml", "not valid toml [[[\n")
    .check(
        "config-error-envelope-carries-remediation-hint",
        error_envelope_with_remediation_hint(0, "wai doctor"),
    )
    .check(
        "agent-followed-doctor-hint",
        agent_followed_hint_loose(1, "wai doctor"),
    )
    .check("agent-executed-all", agent_executed_all())
    .check("status-recovery-ok", ok_envelope_loose(3))
}

/// The dont state-machine lifecycle scenario (evallerina-ey7
/// hint-adherence family).
///
/// Contrived failures: the dont claim state machine's guard rails, each
/// of which carries a remediation hint the agent must consume. Recorded
/// live against dont 0.2.2
/// (`scenarios/hint-adherence/dont-hint-adherence-lifecycle.json`):
/// 1. `dont init --json`, `dont conclude "…" --json` — claim created
///    (unverified)
/// 2. `dont trust <id>` without --reason — error `reason-required` whose
///    remediation names the full command shape; the agent re-runs with a
///    concrete reason → unverified → doubted
/// 3. `dont flag <id>` without evidence — error `no-evidence`; the agent
///    retries with a `file://` URI which the tool itself rejects
///    (malformed-evidence-uri, remediation names `<http://…>`); the
///    agent consumes *that* hint too and retries with `https://` →
///    doubted → verified
/// 4. `dont show <id>` — data.status `verified`
///
/// The claim id is the one minted at capture time and is recorded
/// verbatim (tier-1 replay is offline; ids are stable within the
/// recording). Checks, one fault each:
/// - `trust-error-carries-reason-remediation` (tool fault)
/// - `agent-followed-reason-hint` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `flag-error-carries-evidence-remediation` (tool fault)
/// - `agent-followed-evidence-hint` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `file-uri-rejected-with-repair-hint` (tool fault)
/// - `agent-followed-uri-repair-hint` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `show-reports-verified` (tool fault if the state machine did not
///   land on verified)
pub fn dont_lifecycle_scenario() -> Scenario {
    const CLAIM: &str = "claim:01M4GRB4M65K86K518SK38FSMQ";
    Scenario::new(
        "dont-hint-adherence-lifecycle",
        "Record and verify a claim about the nightly sync.",
    )
    .check(
        "trust-error-carries-reason-remediation",
        error_envelope_with_hint_containing(2, format!("dont trust {CLAIM} --reason")),
    )
    .check(
        "agent-followed-reason-hint",
        agent_followed_hint_loose(3, format!("dont trust {CLAIM} --reason")),
    )
    .check(
        "flag-error-carries-evidence-remediation",
        error_envelope_with_hint_containing(4, format!("dont flag {CLAIM} --evidence")),
    )
    .check(
        "agent-followed-evidence-hint",
        agent_followed_hint_loose(5, format!("dont flag {CLAIM} --evidence")),
    )
    .check(
        "file-uri-rejected-with-repair-hint",
        error_envelope_with_hint_containing(5, "--evidence <http://...>"),
    )
    .check(
        "agent-followed-uri-repair-hint",
        agent_followed_hint_loose(6, format!("dont flag {CLAIM} --evidence")),
    )
    .check("agent-executed-all", agent_executed_all())
    .check(
        "show-reports-verified",
        envelope_field_is(7, "/data/status", "verified"),
    )
}
