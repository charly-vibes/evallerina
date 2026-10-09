//! Purpose: Tier-1 scripted-replay tests for the evallerina harness.
//! Responsibilities: replay recorded AgentStep trajectories through
//! `genesis::evals::Scenario::run` and assert the deterministic checks
//! detect both adherence (pass) and drift (fail with the right taxonomy).
//! Rationale: the evals-guidelines tier ladder — tier 1 proves recorded
//! failure modes are still detected, with zero model calls and zero
//! network; `just tier1` gates nightly on this file.

use evallerina::envelope::agent_followed_hint_loose;
use evallerina::recorded::RecordedTrajectory;
use evallerina::scenario::smoke_scenario;
use genesis::evals::{AgentStep, Scenario, agent_executed_all};

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
