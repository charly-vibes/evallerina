//! Purpose: Tier-1 scripted-replay tests for the evallerina harness.
//! Responsibilities: replay recorded AgentStep trajectories through
//! `genesis::evals::Scenario::run` and assert the deterministic checks
//! detect both adherence (pass) and drift (fail with the right taxonomy).
//! Rationale: the evals-guidelines tier ladder — tier 1 proves recorded
//! failure modes are still detected, with zero model calls and zero
//! network; `just tier1` gates nightly on this file.

use evallerina::envelope::agent_followed_hint_loose;
use evallerina::recorded::RecordedTrajectory;
use evallerina::scenario::{
    HINT_ADHERENCE_DIR, dont_lifecycle_scenario, smoke_scenario, wai_corrupt_config_scenario,
    wai_typo_scenario,
};
use genesis::evals::{AgentStep, Scenario, agent_executed_all};

/// Load a recorded trajectory from `scenarios/hint-adherence/` by name.
fn hint_adherence_trajectory(name: &str) -> RecordedTrajectory {
    let path = format!("{HINT_ADHERENCE_DIR}/{name}.json");
    RecordedTrajectory::from_json(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("recorded trajectory {path}: {e}")),
    )
    .unwrap_or_else(|e| panic!("recorded trajectory {path} parses: {e}"))
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
    .check(
        "agent-followed-hint",
        agent_followed_hint_loose(1, "wai doctor"),
    )
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
            agent_followed_hint_loose(1, "wai status"),
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
        agent_followed_hint_loose(1, "wai doctor"),
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
        agent_followed_hint_loose(3, "dont trust claim:01M4GRB4M65K86K518SK38FSMQ --reason"),
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
