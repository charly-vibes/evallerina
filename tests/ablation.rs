//! Purpose: Contract tests for the A/B ablation harness (evallerina-rl1,
//! evals.md Step 5).
//! Responsibilities: pin the A/B shape — full arm (llms.txt + managed
//! AGENTS.md + .genesis/tools.toml provisioned into the fixture) vs
//! ablated arm (raw binaries, no AIX context), delta computed over ≥3
//! runs per arm, absent and rate-limited cells surfaced without being
//! counted as failures, arm recorded additively on tier-2 rows
//! (report_version 1 unchanged), and loud refusal on artifact-path
//! collisions instead of silently overwriting scenario fixtures.
//! Rationale: the score delta is the measured value of the AIX
//! investment; a harness that fakes a delta (undefined → 0, dropped
//! cells, sub-3 samples) would manufacture value the evals never
//! measured.

use genesis::evals::{CheckOutcome, Scenario};
use serde_json::json;

use evallerina::ablation::{AblationFactories, AixArtifacts, run_ablation};
use evallerina::live::{Arm, Bounds, RotationEvent, Transport, TransportError, TrialStatus};

/// The dulce-family AIX artifact set (test content; production wires
/// the real generated artifacts).
fn test_artifacts() -> AixArtifacts {
    AixArtifacts {
        llms_txt: "llms.txt content: wai status --json",
        managed_agents_md: "<!-- WAI:START -->managed guidance<!-- WAI:END -->",
        tools_toml: "[tools]\nwai = { cmd = \"wai\" }\n",
    }
}

/// Probe scenario: the check passes iff the artifact `llms.txt` is
/// provisioned in the fixture — so the full arm passes and the ablated
/// arm fails, making the delta fully scripted and deterministic.
fn artifacts_scenario() -> Scenario {
    Scenario::new("ablation-probe", "create the file").check("artifacts-present", |r| {
        if r.fixture_root.join("llms.txt").exists() {
            CheckOutcome::pass()
        } else {
            CheckOutcome::tool_fault("llms.txt missing from fixture")
        }
    })
}

/// A transport with a scripted completion list.
struct FakeTransport {
    completions: Vec<Result<String, TransportError>>,
}

impl FakeTransport {
    fn new(completions: Vec<Result<String, TransportError>>) -> Self {
        Self { completions }
    }
}

impl Transport for FakeTransport {
    fn complete(
        &mut self,
        _messages: &[evallerina::live::Message],
    ) -> Result<String, TransportError> {
        if self.completions.is_empty() {
            return Err(TransportError::Http("script exhausted".into()));
        }
        self.completions.remove(0)
    }
}

fn action(command: &str, done: bool) -> String {
    json!({ "command": command, "done": done }).to_string()
}

/// A green two-turn script: run a command, then declare done.
fn green_turns() -> Vec<Result<String, TransportError>> {
    vec![
        Ok(action("printf ok > touched.txt", false)),
        Ok(action("true", true)),
    ]
}

/// A green script sized for `trials` two-turn trials (the factory is
/// called once per model, and one transport serves the whole cell).
fn green_script(trials: u32) -> Vec<Result<String, TransportError>> {
    let mut completions = Vec::new();
    for _ in 0..trials {
        completions.extend(green_turns());
    }
    completions
}

/// Factory whose transports complete every task green.
fn green_factory() -> impl FnMut(&str) -> Result<Box<dyn Transport>, String> {
    move |_model| Ok(Box::new(FakeTransport::new(green_script(3))) as Box<dyn Transport>)
}

/// Factory whose transports are always rate-limited (429, retry also
/// 429 → each trial ends `rate_limited`, no fault attribution).
fn rate_limited_factory() -> impl FnMut(&str) -> Result<Box<dyn Transport>, String> {
    move |_model| {
        Ok(Box::new(FakeTransport::new(vec![
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
        ])) as Box<dyn Transport>)
    }
}

fn always_open() -> bool {
    true
}

/// A shared budget that stays open for `n` trial starts, then closes
/// (deterministic mid-ablation expiry without sleeping).
fn budget_closes_after(n: u32) -> impl Fn() -> bool {
    let remaining = std::cell::Cell::new(n);
    move || {
        if remaining.get() == 0 {
            return false;
        }
        remaining.set(remaining.get() - 1);
        true
    }
}

// ── the A/B shape ───────────────────────────────────────────────────────────

/// The Must gate: an A/B report for one scenario with the delta
/// computed over ≥3 runs per arm. The full arm passes because the
/// artifacts are provisioned; the ablated arm fails the same check —
/// the delta is entirely the artifact effect.
#[test]
fn delta_computed_over_three_runs_per_arm() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 3);
    assert_eq!(report.ablated.failed, 3);
    assert_eq!(report.delta, Some(1.0));
}

/// The full arm sees the three AIX artifacts in its fixture; the
/// ablated arm sees none (raw binaries only).
#[test]
fn artifact_fixture_files_map_the_three_paths() {
    let files = test_artifacts().fixture_files();
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths, ["AGENTS.md", ".genesis/tools.toml", "llms.txt"]);
}

// ── anti-goals: no manufactured deltas ──────────────────────────────────────

/// The Must gate floor is three runs per arm; fewer samples cannot
/// support a delta claim. Refuse loudly, never run a thin A/B.
#[test]
fn reps_below_three_is_an_error() {
    let err = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        2,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect_err("reps < 3 refused");
    assert!(err.contains("3"), "error names the floor: {err}");
}

/// An arm where nothing ran has no pass rate: the delta is undefined,
/// recorded as None — never silently zero, which would read as "no
/// measured difference".
#[test]
fn delta_undefined_when_an_arm_cannot_run() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut rate_limited_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.ablated.trials, 3);
    assert_eq!(report.ablated.rate_limited, 3);
    assert_eq!(report.ablated.pass_rate, None);
    assert_eq!(report.delta, None);
}

/// Budget-skipped cells are recorded absent, never failed, and are
/// excluded from the pass-rate denominator while still being surfaced.
#[test]
fn absent_cells_surfaced_and_excluded_from_delta_denominator() {
    // 3 budget ticks for the full arm, 1 for the ablated arm; the
    // remaining 2 ablated cells never start → absent.
    let budget = budget_closes_after(4);
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &budget,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 3);
    assert_eq!(report.ablated.trials, 1);
    assert_eq!(report.ablated.failed, 1);
    assert_eq!(report.ablated.absent, 2);
    assert_eq!(report.delta, Some(1.0));
}

// ── arm attribution on tier-2 rows (additive, version unchanged) ────────────

/// Tier-2 rows record the arm so concatenated rotation + ablation
/// output stays attributable; the field is additive, report_version
/// stays 1 (evals-guidelines additive evolution).
#[test]
fn rows_carry_arm_and_report_version_stays_1() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    for event in &report.full_rows {
        match event {
            RotationEvent::Trial { report } => {
                assert_eq!(report.arm, Arm::Full);
                assert_eq!(report.report_version, 1);
            }
            RotationEvent::Absent { arm, .. } => assert_eq!(*arm, Arm::Full),
        }
    }
    for event in &report.ablated_rows {
        match event {
            RotationEvent::Trial { report } => assert_eq!(report.arm, Arm::Ablated),
            RotationEvent::Absent { arm, .. } => assert_eq!(*arm, Arm::Ablated),
        }
    }
}

// ── fixture integrity ───────────────────────────────────────────────────────

/// Provisioning never silently overwrites scenario material: an
/// artifact path colliding with a scenario fixture is a loud error —
/// the agent must not observe an artifact where the scenario injected
/// a fault (e.g. a corrupt config).
#[test]
fn artifact_path_collision_is_a_loud_error() {
    let scenario = Scenario::new("colliding", "create the file")
        .fixture_file("AGENTS.md", "scenario-owned content")
        .check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    let err = run_ablation(
        &scenario,
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect_err("collision refused");
    assert!(
        err.contains("AGENTS.md"),
        "error names the colliding path: {err}"
    );
}

// ── report shape ────────────────────────────────────────────────────────────

/// The A/B report serializes with per-arm detail and its own version;
/// the only score is the per-scenario delta — no single aggregate
/// number across scenarios exists.
#[test]
fn ablation_report_serializes_with_per_arm_detail() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    let rendered = serde_json::to_value(&report).expect("serializes");
    assert_eq!(rendered["ablation_version"], 1);
    assert_eq!(rendered["scenario"], "ablation-probe");
    assert_eq!(rendered["full"]["arm"], "full");
    assert_eq!(rendered["ablated"]["arm"], "ablated");
    assert_eq!(rendered["full"]["pass_rate"], 1.0);
    assert_eq!(rendered["delta"], 1.0);
    assert!(rendered.get("score").is_none());
    assert!(rendered.get("aggregate").is_none());
    // The report is auditable: it carries the underlying rows verbatim.
    assert_eq!(rendered["full_rows"].as_array().map(Vec::len), Some(3));
    assert_eq!(rendered["ablated_rows"].as_array().map(Vec::len), Some(3));
}

/// A mixed delta stays exact: full 2/3 vs ablated 0/3 → 2/3. This
/// probe's check depends on the agent's action (touched.txt), so a
/// scripted failure inside the full arm is observable.
#[test]
fn mixed_results_produce_fractional_delta() {
    let action_scenario =
        Scenario::new("ablation-probe-action", "create the file").check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    // Script trial 2's turns to declare done without ever writing the
    // file; trials 1 and 3 stay green. Turn layout: trial n owns
    // completions [2n, 2n+1] — overwrite both of trial 2's turns.
    let mut full_factory = move |_model: &str| {
        let mut scripted = green_script(3);
        scripted[2] = Ok(action("true", false));
        scripted[3] = Ok(action("true", true));
        Ok(Box::new(FakeTransport::new(scripted)) as Box<dyn Transport>)
    };
    let report = run_ablation(
        &action_scenario,
        &["m1".to_owned()],
        3,
        &test_artifacts(),
        AblationFactories {
            full: &mut full_factory,
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 2);
    assert_eq!(report.full.failed, 1);
    assert_eq!(report.ablated.passed, 3);
    let delta = report.delta.expect("both arms ran");
    assert!((delta - (-1.0 / 3.0)).abs() < 1e-9);
}

/// `Arm` serializes snake_case and the plain (non-ablation) rotation
/// rows attribute to the full arm by default.
#[test]
fn plain_rotation_rows_default_to_full_arm() {
    use evallerina::live::{TransportFactory, run_rotation};
    let action_scenario =
        Scenario::new("ablation-probe-action", "create the file").check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    let mut factory: TransportFactory =
        &mut |_model| Ok(Box::new(FakeTransport::new(green_script(1))) as Box<dyn Transport>);
    let events = run_rotation(
        &[action_scenario],
        &["m1".to_owned()],
        1,
        &mut factory,
        &Bounds::default(),
        &always_open,
    )
    .expect("rotation runs");
    match &events[0] {
        RotationEvent::Trial { report } => {
            assert_eq!(report.arm, Arm::Full);
            assert!(matches!(report.status, TrialStatus::Passed));
        }
        RotationEvent::Absent { .. } => panic!("open budget never skips"),
    }
}
