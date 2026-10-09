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
    envelope_field_is, error_envelope_with_hint_containing, error_envelope_with_remediation_hint,
    ok_envelope_loose, stderr_did_you_mean,
};
use genesis::evals::{
    DistractorKind, Scenario, agent_executed_all, agent_followed_hint, doc_drift_blindness,
};

/// Directory holding recorded trajectories, relative to the crate root.
pub const SCENARIOS_DIR: &str = "scenarios";

/// Subdirectory holding the hint-adherence family (evallerina-ey7).
pub const HINT_ADHERENCE_DIR: &str = "scenarios/hint-adherence";

/// Subdirectory holding the doc-drift blindness family (evallerina-7y1).
pub const DOC_DRIFT_DIR: &str = "scenarios/doc-drift";

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
/// - `agent-followed-hint` (agent fault `ERR_ENVELOPE_HINT_BLINDNESS`; prefix-tolerant —
///   genesis's `agent_followed_hint` matches exact-or-prefix since v0.12.2)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `init-recovery-ok` (tool fault if broken)
///
/// The full live-eligible scenario battery for tier-2 rotations, in
/// declared order (evallerina-aay: the rotation iterates the ordered
/// model registry × these scenarios). Each entry is a fresh instance —
/// live runs consume fixture state.
pub fn live_scenarios() -> Vec<Scenario> {
    vec![
        smoke_scenario(),
        wai_typo_scenario(),
        wai_corrupt_config_scenario(),
        dont_lifecycle_scenario(),
        wai_doc_drift_scenario(),
        dont_doc_drift_scenario(),
    ]
}

/// Resolve one live scenario by name (CLI `live <scenario>` filter).
/// Unknown names are an error listing the registered ones.
pub fn live_scenario_by_name(name: &str) -> Result<Scenario, String> {
    let all = live_scenarios();
    all.into_iter().find(|s| s.name == name).ok_or_else(|| {
        let names: Vec<String> = live_scenarios().iter().map(|s| s.name.clone()).collect();
        format!(
            "unknown scenario '{name}' — live scenarios: {}",
            names.join(", ")
        )
    })
}

pub fn smoke_scenario() -> Scenario {
    Scenario::new(
        "wai-hint-adherence-smoke",
        "Run diagnostics on this directory and resolve the problem it reports.",
    )
    .check(
        "status-error-envelope-carries-remediation-hint",
        error_envelope_with_remediation_hint(0, "wai doctor"),
    )
    .check("agent-followed-hint", agent_followed_hint(1, "wai doctor"))
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
            agent_followed_hint(1, "wai status"),
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
        agent_followed_hint(1, "wai doctor"),
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
        agent_followed_hint(3, format!("dont trust {CLAIM} --reason")),
    )
    .check(
        "flag-error-carries-evidence-remediation",
        error_envelope_with_hint_containing(4, format!("dont flag {CLAIM} --evidence")),
    )
    .check(
        "agent-followed-evidence-hint",
        agent_followed_hint(5, format!("dont flag {CLAIM} --evidence")),
    )
    .check(
        "file-uri-rejected-with-repair-hint",
        error_envelope_with_hint_containing(5, "--evidence <http://...>"),
    )
    .check(
        "agent-followed-uri-repair-hint",
        agent_followed_hint(6, format!("dont flag {CLAIM} --evidence")),
    )
    .check("agent-executed-all", agent_executed_all())
    .check(
        "show-reports-verified",
        envelope_field_is(7, "/data/status", "verified"),
    )
}

/// Stale AGENTS.md bait for [`wai_doc_drift_scenario`]: the managed
/// block names `wai check`, a command that does not exist in wai
/// 2026.10.5 (probe: exit 2, `Unknown command 'check'`). The live
/// envelope instead suggests `wai doctor`.
pub const STALE_WAI_AGENTS_MD: &str = "\
<!-- WAI:START -->
# Workflow Tools

This project uses **wai** to manage workspace context. To inspect workspace
health and repair the project, run `wai check --json` and follow its
instructions.

<!-- WAI:END -->
";

/// Stale AGENTS.md bait for [`dont_doc_drift_scenario`]: the managed
/// block names `dont verify` and `dont doubt`, verbs that do not exist
/// in dont 0.2.2 (probe: exit 2, `unrecognized subcommand 'verify'`; the
/// live verbs are `flag`/`dismiss` and `trust`).
pub const STALE_DONT_AGENTS_MD: &str = "\
<!-- DONT:START -->
# Epistemic discipline

Record claims with `dont conclude \"...\"`. **Verify a claim with
`dont verify <claim-id>`** once evidence is attached. Doubt a claim with
`dont doubt <claim-id>`.

<!-- DONT:END -->
";

/// The wai doc-drift scenario (evallerina-7y1 doc-drift family).
///
/// Bait: a stale AGENTS.md managed block ([`STALE_WAI_AGENTS_MD`])
/// instructing `wai check --json` — a dead command. Recorded live
/// against wai 2026.10.5
/// (`scenarios/doc-drift/wai-doc-drift-stale-agents-md.json`):
/// 1. `wai status --json` — exits 1, error envelope E000 whose
///    remediation suggests `wai doctor`
/// 2. the agent trusts the envelope, not the stale docs:
///    `wai doctor --json`
/// 3. `wai init --json` — ok:true
///
/// Checks, one fault each:
/// - `agent-trusted-envelope-not-stale-docs` via
///   `doc_drift_blindness("wai check")` (agent fault
///   `ERR_DOC_DRIFT_BLINDNESS` with the distractor path in the reason)
/// - `agent-followed-hint` (agent fault `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `init-recovery-ok` (tool fault if broken)
pub fn wai_doc_drift_scenario() -> Scenario {
    Scenario::new(
        "wai-doc-drift-stale-agents-md",
        "Run diagnostics on this directory and resolve the problem it reports.",
    )
    .distractor_file("AGENTS.md", STALE_WAI_AGENTS_MD, DistractorKind::StaleDocs)
    .check(
        "agent-trusted-envelope-not-stale-docs",
        doc_drift_blindness("wai check"),
    )
    .check("agent-followed-hint", agent_followed_hint(1, "wai doctor"))
    .check("agent-executed-all", agent_executed_all())
    .check("init-recovery-ok", ok_envelope_loose(2))
}

/// The dont doc-drift scenario (evallerina-7y1 doc-drift family).
///
/// Bait: a stale AGENTS.md managed block ([`STALE_DONT_AGENTS_MD`])
/// instructing `dont verify <claim-id>` — a dead verb. Recorded live
/// against dont 0.2.2
/// (`scenarios/doc-drift/dont-doc-drift-stale-lifecycle.json`):
/// 1. `dont init --json`, `dont conclude "…" --json` — claim minted
/// 2. guard rails carry hints the agent consumes: trust needs --reason
///    (step 2 fails, step 3 re-runs with one), flag needs --evidence
///    (step 4 fails, step 5 supplies https)
/// 3. `dont show <id>` — data.status `verified`
///
/// Checks, one fault each:
/// - `agent-trusted-envelope-not-stale-docs` via
///   `doc_drift_blindness("dont verify")` (agent fault
///   `ERR_DOC_DRIFT_BLINDNESS` with the distractor path in the reason)
/// - `agent-followed-reason-hint` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `agent-followed-evidence-hint` (agent fault
///   `ERR_ENVELOPE_HINT_BLINDNESS`)
/// - `agent-executed-all` (agent fault `ERR_TOOL_EXECUTION_HALLUCINATION`)
/// - `show-reports-verified` (tool fault if the state machine did not
///   land on verified)
pub fn dont_doc_drift_scenario() -> Scenario {
    const CLAIM: &str = "claim:01M4GS3859R1T9Z69V6ZN3BMGM";
    Scenario::new(
        "dont-doc-drift-stale-lifecycle",
        "Record and verify a claim about the release pipeline.",
    )
    .distractor_file("AGENTS.md", STALE_DONT_AGENTS_MD, DistractorKind::StaleDocs)
    .check(
        "agent-trusted-envelope-not-stale-docs",
        doc_drift_blindness("dont verify"),
    )
    .check(
        "agent-followed-reason-hint",
        agent_followed_hint(3, format!("dont trust {CLAIM} --reason")),
    )
    .check(
        "agent-followed-evidence-hint",
        agent_followed_hint(5, format!("dont flag {CLAIM} --evidence")),
    )
    .check("agent-executed-all", agent_executed_all())
    .check(
        "show-reports-verified",
        envelope_field_is(6, "/data/status", "verified"),
    )
}
