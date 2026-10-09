//! Purpose: Tier-1 scripted-replay tests for the evallerina harness.
//! Responsibilities: replay recorded AgentStep trajectories through
//! `genesis::evals::Scenario::run` and assert the deterministic checks
//! detect both adherence (pass) and drift (fail with the right taxonomy).
//! Rationale: the evals-guidelines tier ladder — tier 1 proves recorded
//! failure modes are still detected, with zero model calls and zero
//! network; `just tier1` gates nightly on this file.

use evallerina::recorded::RecordedTrajectory;
use evallerina::scenario::{
    DOC_DRIFT_DIR, HINT_ADHERENCE_DIR, STALE_DONT_AGENTS_MD, STALE_WAI_AGENTS_MD,
    dont_doc_drift_scenario, dont_lifecycle_scenario, smoke_scenario, wai_corrupt_config_scenario,
    wai_doc_drift_scenario, wai_typo_scenario,
};
use genesis::evals::{AgentStep, Scenario, agent_executed_all, agent_followed_hint};

/// Rewrite every occurrence of a recorded claim id with a fresh one —
/// in both the agent's commands and the tool's captured stdout — to
/// simulate a *live* replay where the agent minted its own id instead
/// of reproducing the recorded one (evallerina-hhs).
fn with_fresh_claim_id(
    trajectory: &RecordedTrajectory,
    recorded_id: &str,
    fresh_id: &str,
) -> Vec<AgentStep> {
    trajectory
        .steps()
        .iter()
        .map(|s| AgentStep {
            command: s.command.replace(recorded_id, fresh_id),
            stdout: s.stdout.replace(recorded_id, fresh_id),
            stderr: s.stderr.clone(),
            exit_code: s.exit_code,
            executed: s.executed,
        })
        .collect()
}

/// Load a recorded trajectory from a scenario directory by name.
fn recorded_trajectory(dir: &str, name: &str) -> RecordedTrajectory {
    let path = format!("{dir}/{name}.json");
    RecordedTrajectory::from_json(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("recorded trajectory {path}: {e}")),
    )
    .unwrap_or_else(|e| panic!("recorded trajectory {path} parses: {e}"))
}

/// Load a recorded trajectory from `scenarios/hint-adherence/` by name.
fn hint_adherence_trajectory(name: &str) -> RecordedTrajectory {
    recorded_trajectory(HINT_ADHERENCE_DIR, name)
}

/// Load a recorded trajectory from `scenarios/doc-drift/` by name.
fn doc_drift_trajectory(name: &str) -> RecordedTrajectory {
    recorded_trajectory(DOC_DRIFT_DIR, name)
}

/// The smoke Must gate (evallerina-e10): one recorded wai trajectory
/// replayed green. Proves the harness end-to-end — fixture materializes,
/// envelopes parse, the agent followed the remediation hint, recovery
/// emitted ok:true.
#[test]
fn wai_hint_adherence_smoke_replays_green() {
    let trajectory = RecordedTrajectory::from_json(
        &std::fs::read_to_string("scenarios/wai-hint-adherence-smoke.json")
            .expect("recorded trajectory exists"),
    )
    .expect("recorded trajectory parses");
    let scenario = smoke_scenario();
    let report = scenario
        .run(trajectory.steps())
        .expect("fixture materializes");
    assert!(
        report.passed,
        "smoke scenario must pass, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}

/// The same scenario must *detect* hint blindness: replay a doctored
/// transcript where the agent ignored the `wai doctor` remediation hint
/// and brute-forced `wai status` again instead. The battery's value is
/// that this fails with `ERR_ENVELOPE_HINT_BLINDNESS`, not silently
/// passes.
#[test]
fn hint_blind_replay_is_detected() {
    let trajectory = RecordedTrajectory::from_json(
        &std::fs::read_to_string("scenarios/wai-hint-adherence-smoke.json").unwrap(),
    )
    .unwrap();
    let mut steps: Vec<AgentStep> = trajectory.steps();
    // Blind agent: after the failed status, run status again instead of
    // the suggested `wai doctor`, then init.
    steps[1].command = "wai status --json".to_string();

    let scenario = Scenario::new(
        "wai-hint-adherence-smoke",
        "Recover from the status failure.",
    )
    .check("agent-followed-hint", agent_followed_hint(1, "wai doctor"))
    .check("agent-executed-all", agent_executed_all());
    let report = scenario.run(steps).expect("fixture materializes");
    assert!(!report.passed, "hint-blind replay must fail");
    let codes: Vec<Option<&str>> = report
        .failures
        .iter()
        .map(|(name, outcome)| {
            assert_eq!(name, "agent-followed-hint", "only the hint check may fail");
            match outcome {
                genesis::evals::CheckOutcome::Fail { taxonomy, .. } => taxonomy.map(|t| t.code()),
                genesis::evals::CheckOutcome::Pass => None,
            }
        })
        .collect();
    assert_eq!(
        codes,
        vec![Some("ERR_ENVELOPE_HINT_BLINDNESS")],
        "blindness must classify as ERR_ENVELOPE_HINT_BLINDNESS"
    );
}

// ---------------------------------------------------------------------
// evallerina-ey7: hint-adherence family under scenarios/hint-adherence/
// ---------------------------------------------------------------------

/// Must gate (evallerina-ey7): the wai DidYouMean typo scenario replays
/// green — the tool's stderr hint channel is intact and the recorded
/// agent consumed the suggestion (`wai statuss` → `wai status`).
#[test]
fn wai_hint_adherence_typo_replays_green() {
    let trajectory = hint_adherence_trajectory("wai-hint-adherence-typo");
    let report = wai_typo_scenario()
        .run(trajectory.steps())
        .expect("fixture materializes");
    assert!(
        report.passed,
        "typo scenario must pass, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}

/// The typo scenario must *detect* hint blindness: an agent that
/// re-issues the same misspelled command instead of the suggested
/// `wai status` fails with `ERR_ENVELOPE_HINT_BLINDNESS`.
#[test]
fn typo_blind_replay_is_detected() {
    let trajectory = hint_adherence_trajectory("wai-hint-adherence-typo");
    let mut steps: Vec<AgentStep> = trajectory.steps();
    steps[1].command = "wai statuss --json".to_string();

    let scenario = Scenario::new("wai-hint-adherence-typo", "Run status on this directory.")
        .check(
            "agent-followed-did-you-mean",
            agent_followed_hint(1, "wai status"),
        )
        .check("agent-executed-all", agent_executed_all());
    let report = scenario.run(steps).expect("fixture materializes");
    assert!(!report.passed, "blind typo replay must fail");
    let codes: Vec<Option<&str>> = report
        .failures
        .iter()
        .map(|(name, outcome)| {
            assert_eq!(
                name, "agent-followed-did-you-mean",
                "only the hint check may fail"
            );
            match outcome {
                genesis::evals::CheckOutcome::Fail { taxonomy, .. } => taxonomy.map(|t| t.code()),
                genesis::evals::CheckOutcome::Pass => None,
            }
        })
        .collect();
    assert_eq!(codes, vec![Some("ERR_ENVELOPE_HINT_BLINDNESS")]);
}

/// Must gate (evallerina-ey7): the corrupt-config scenario replays
/// green — the E000 config envelope suggests `wai doctor`, the recorded
/// agent followed it, repaired the config, and status recovered with
/// ok:true.
#[test]
fn wai_hint_adherence_corrupt_config_replays_green() {
    let trajectory = hint_adherence_trajectory("wai-hint-adherence-corrupt-config");
    let report = wai_corrupt_config_scenario()
        .run(trajectory.steps())
        .expect("fixture materializes");
    assert!(
        report.passed,
        "corrupt-config scenario must pass, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}

/// The corrupt-config scenario must *detect* hint blindness: an agent
/// that reaches for `wai init` instead of the suggested `wai doctor`.
/// Notably, init returns ok:true *without repairing* the corrupt config
/// — a hint-blind agent is rewarded with a green envelope; only the
/// hint check catches the drift.
#[test]
fn corrupt_config_blind_replay_is_detected() {
    let trajectory = hint_adherence_trajectory("wai-hint-adherence-corrupt-config");
    let mut steps: Vec<AgentStep> = trajectory.steps();
    steps[1].command = "wai init --json".to_string();

    let scenario = Scenario::new(
        "wai-hint-adherence-corrupt-config",
        "Recover from the status failure.",
    )
    .check(
        "agent-followed-doctor-hint",
        agent_followed_hint(1, "wai doctor"),
    )
    .check("agent-executed-all", agent_executed_all());
    let report = scenario.run(steps).expect("fixture materializes");
    assert!(!report.passed, "blind corrupt-config replay must fail");
    let codes: Vec<Option<&str>> = report
        .failures
        .iter()
        .map(|(name, outcome)| {
            assert_eq!(
                name, "agent-followed-doctor-hint",
                "only the hint check may fail"
            );
            match outcome {
                genesis::evals::CheckOutcome::Fail { taxonomy, .. } => taxonomy.map(|t| t.code()),
                genesis::evals::CheckOutcome::Pass => None,
            }
        })
        .collect();
    assert_eq!(codes, vec![Some("ERR_ENVELOPE_HINT_BLINDNESS")]);
}

/// Must gate (evallerina-ey7): the dont state-machine lifecycle
/// replays green — every guard rail (missing --reason, missing
/// evidence, malformed file:// URI) carried a remediation hint and the
/// recorded agent consumed each one, landing the claim on verified.
#[test]
fn dont_hint_adherence_lifecycle_replays_green() {
    let trajectory = hint_adherence_trajectory("dont-hint-adherence-lifecycle");
    let report = dont_lifecycle_scenario()
        .run(trajectory.steps())
        .expect("fixture materializes");
    assert!(
        report.passed,
        "dont lifecycle scenario must pass, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}

/// The dont lifecycle must *detect* hint blindness: an agent that
/// re-runs `dont trust <id>` without the required --reason instead of
/// consuming the remediation hint.
#[test]
fn dont_lifecycle_blind_replay_is_detected() {
    let trajectory = hint_adherence_trajectory("dont-hint-adherence-lifecycle");
    let mut steps: Vec<AgentStep> = trajectory.steps();
    steps[3].command = steps[2].command.clone();

    let scenario = Scenario::new(
        "dont-hint-adherence-lifecycle",
        "Record and verify a claim about the nightly sync.",
    )
    .check(
        "agent-followed-reason-hint",
        agent_followed_hint(3, "dont trust claim:01M4GRB4M65K86K518SK38FSMQ --reason"),
    )
    .check("agent-executed-all", agent_executed_all());
    let report = scenario.run(steps).expect("fixture materializes");
    assert!(!report.passed, "blind dont replay must fail");
    let codes: Vec<Option<&str>> = report
        .failures
        .iter()
        .map(|(name, outcome)| {
            assert_eq!(
                name, "agent-followed-reason-hint",
                "only the hint check may fail"
            );
            match outcome {
                genesis::evals::CheckOutcome::Fail { taxonomy, .. } => taxonomy.map(|t| t.code()),
                genesis::evals::CheckOutcome::Pass => None,
            }
        })
        .collect();
    assert_eq!(codes, vec![Some("ERR_ENVELOPE_HINT_BLINDNESS")]);
}

/// Live-replay seam (evallerina-hhs): a live agent mints its own claim
/// id, so the recorded id never appears in the live transcript. The
/// scenario's checks must still pass — hint-presence checks match the
/// id-agnostic shape of the remediation, hint-following checks match
/// the id-agnostic shape of the executed command.
#[test]
fn dont_lifecycle_live_replay_with_fresh_claim_id_stays_green() {
    let trajectory = hint_adherence_trajectory("dont-hint-adherence-lifecycle");
    let steps = with_fresh_claim_id(
        &trajectory,
        "claim:01M4GRB4M65K86K518SK38FSMQ",
        "claim:01LIVEFRESHCLAIM0123456789ABCDEF",
    );
    let report = dont_lifecycle_scenario()
        .run(steps)
        .expect("fixture materializes");
    assert!(
        report.passed,
        "dont lifecycle must pass with a freshly minted claim id, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}
// ---------------------------------------------------------------------

/// Must gate (evallerina-7y1): the stale-AGENTS.md scenario replays
/// green — the bait (a managed block naming the dead command `wai
/// check`) was materialized and the recorded agent trusted the live
/// envelope (`wai doctor`), never the stale docs.
#[test]
fn wai_doc_drift_stale_agents_md_replays_green() {
    let trajectory = doc_drift_trajectory("wai-doc-drift-stale-agents-md");
    let report = wai_doc_drift_scenario()
        .run(trajectory.steps())
        .expect("fixture materializes");
    assert!(
        report.passed,
        "doc-drift scenario must pass, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}

/// The doc-drift scenario must *detect* blindness: an agent that follows
/// the stale AGENTS.md and runs the dead `wai check` command fails with
/// `ERR_DOC_DRIFT_BLINDNESS`, and the failing reason names the
/// distractor path (Must gate: distractor path present on the failing
/// side).
#[test]
fn wai_doc_drift_blind_replay_is_detected() {
    let trajectory = doc_drift_trajectory("wai-doc-drift-stale-agents-md");
    let mut steps: Vec<AgentStep> = trajectory.steps();
    // Doc-blind agent: run the stale block's `wai check` before trusting
    // the envelope channel.
    steps.insert(
        1,
        AgentStep {
            command: "wai check --json".to_string(),
            stdout: String::new(),
            stderr: "wai: Unknown command 'check'.\n".to_string(),
            exit_code: 2,
            executed: true,
        },
    );

    let scenario = Scenario::new(
        "wai-doc-drift-stale-agents-md",
        "Run diagnostics on this directory and resolve the problem it reports.",
    )
    .distractor_file(
        "AGENTS.md",
        STALE_WAI_AGENTS_MD,
        genesis::evals::DistractorKind::StaleDocs,
    )
    .check(
        "agent-trusted-envelope-not-stale-docs",
        genesis::evals::doc_drift_blindness("wai check"),
    );
    let report = scenario.run(steps).expect("fixture materializes");
    assert!(!report.passed, "doc-blind replay must fail");
    let reasons: Vec<String> = report
        .failures
        .iter()
        .map(|(name, outcome)| {
            assert_eq!(
                name, "agent-trusted-envelope-not-stale-docs",
                "only the doc-drift check may fail"
            );
            match outcome {
                genesis::evals::CheckOutcome::Fail { taxonomy, reason } => {
                    assert_eq!(taxonomy.map(|t| t.code()), Some("ERR_DOC_DRIFT_BLINDNESS"));
                    reason.clone()
                }
                genesis::evals::CheckOutcome::Pass => String::new(),
            }
        })
        .collect();
    assert!(
        reasons.iter().any(|r| r.contains("AGENTS.md")),
        "failure reason must name the distractor path, got: {reasons:?}"
    );
}

/// Must gate (evallerina-7y1): the stale dont-lifecycle scenario
/// replays green — the bait (a managed block naming the dead verb
/// `dont verify`) was materialized and the recorded agent trusted the
/// envelope remediation hints, landing the claim on verified.
#[test]
fn dont_doc_drift_stale_lifecycle_replays_green() {
    let trajectory = doc_drift_trajectory("dont-doc-drift-stale-lifecycle");
    let report = dont_doc_drift_scenario()
        .run(trajectory.steps())
        .expect("fixture materializes");
    assert!(
        report.passed,
        "doc-drift scenario must pass, failures: {}",
        serde_json::to_string(&report).unwrap()
    );
}

/// The dont doc-drift scenario must *detect* blindness: an agent that
/// follows the stale block and runs the dead `dont verify` verb fails
/// with `ERR_DOC_DRIFT_BLINDNESS` naming the distractor path.
#[test]
fn dont_doc_drift_blind_replay_is_detected() {
    let trajectory = doc_drift_trajectory("dont-doc-drift-stale-lifecycle");
    let mut steps: Vec<AgentStep> = trajectory.steps();
    // Doc-blind agent: attempt the stale block's verify verb before
    // trusting the envelope channel.
    steps.insert(
        2,
        AgentStep {
            command: "dont verify claim:01M4GS3859R1T9Z69V6ZN3BMGM".to_string(),
            stdout: String::new(),
            stderr: "error: unrecognized subcommand 'verify'\n".to_string(),
            exit_code: 2,
            executed: true,
        },
    );

    let scenario = Scenario::new(
        "dont-doc-drift-stale-lifecycle",
        "Record and verify a claim about the release pipeline.",
    )
    .distractor_file(
        "AGENTS.md",
        STALE_DONT_AGENTS_MD,
        genesis::evals::DistractorKind::StaleDocs,
    )
    .check(
        "agent-trusted-envelope-not-stale-docs",
        genesis::evals::doc_drift_blindness("dont verify"),
    );
    let report = scenario.run(steps).expect("fixture materializes");
    assert!(!report.passed, "doc-blind replay must fail");
    let reasons: Vec<String> = report
        .failures
        .iter()
        .map(|(name, outcome)| {
            assert_eq!(
                name, "agent-trusted-envelope-not-stale-docs",
                "only the doc-drift check may fail"
            );
            match outcome {
                genesis::evals::CheckOutcome::Fail { taxonomy, reason } => {
                    assert_eq!(taxonomy.map(|t| t.code()), Some("ERR_DOC_DRIFT_BLINDNESS"));
                    reason.clone()
                }
                genesis::evals::CheckOutcome::Pass => String::new(),
            }
        })
        .collect();
    assert!(
        reasons.iter().any(|r| r.contains("AGENTS.md")),
        "failure reason must name the distractor path, got: {reasons:?}"
    );
}
