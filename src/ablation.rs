//! Purpose: A/B ablation harness for the AIX investment (evallerina-rl1,
//! evals.md Step 5).
//! Responsibilities: run one scenario twice per model — a full arm with
//! the AIX artifacts (llms.txt, managed AGENTS.md, `.genesis/tools.toml`)
//! provisioned into the fixture, and an ablated arm with raw binaries
//! only — compute the pass-rate delta over ≥3 runs per arm, and report
//! per-arm counts with absent and rate-limited cells surfaced but never
//! counted as failures.
//! Rationale: the delta is the measured value of the AIX investment.
//! The harness never manufactures it: undefined deltas stay `None`
//! (never silently zero), sub-3 samples are refused loudly, and
//! artifact paths that would overwrite scenario-injected faults are a
//! loud error.

use serde::Serialize;

use genesis::evals::Scenario;

use crate::live::{Arm, ArmContext, Bounds, RotationEvent, TrialStatus, run_rotation_with_arm};

/// Per-arm transport sourcing for an ablation: one factory per arm
/// (the arm is the caller's knowledge, not the trial's).
pub struct AblationFactories<'a> {
    /// Transports for the full arm (AIX artifacts present).
    pub full: crate::live::TransportFactory<'a>,
    /// Transports for the ablated arm (raw binaries).
    pub ablated: crate::live::TransportFactory<'a>,
}

/// Version of the ablation report shape. Additive evolution only
/// within a version; breaking changes bump this.
pub const ABLATION_VERSION: u32 = 1;

/// The evals.md Step 5 sample floor: a delta claim needs ≥3 runs per
/// arm; fewer samples cannot support it.
const REPS_FLOOR: u32 = 3;

/// The AIX artifact set the full arm provisions (dulce tool family:
/// llms.txt + managed AGENTS.md + `.genesis/tools.toml`). Contents are
/// caller-supplied so tests stay hermetic; production wires the real
/// generated artifacts.
#[derive(Debug, Clone, Copy)]
pub struct AixArtifacts {
    /// The `llms.txt` file content.
    pub llms_txt: &'static str,
    /// The managed `AGENTS.md` content (managed blocks included).
    pub managed_agents_md: &'static str,
    /// The `.genesis/tools.toml` content.
    pub tools_toml: &'static str,
}

impl AixArtifacts {
    /// The artifact set as fixture files, written into the full arm's
    /// fixture after the scenario's own material.
    pub fn fixture_files(&self) -> Vec<(String, String)> {
        vec![
            ("AGENTS.md".to_owned(), self.managed_agents_md.to_owned()),
            (".genesis/tools.toml".to_owned(), self.tools_toml.to_owned()),
            ("llms.txt".to_owned(), self.llms_txt.to_owned()),
        ]
    }
}

/// Per-arm outcome summary. `pass_rate` is over trials that ran
/// (`passed + failed + invalid_output`); rate-limited and absent cells
/// are surfaced in their own counters and excluded from the
/// denominator — they are records, never failures.
#[derive(Debug, Clone, Serialize)]
pub struct ArmSummary {
    /// The arm this summary describes.
    pub arm: Arm,
    /// Trials that actually ran.
    pub trials: usize,
    /// Trials where every declared check passed.
    pub passed: usize,
    /// Trials that ended `failed` (a check failed or a bound was
    /// reached).
    pub failed: usize,
    /// Trials that ended `invalid_output` (agent fault).
    pub invalid_output: usize,
    /// Trials that ended `rate_limited` (transport 429 twice).
    pub rate_limited: usize,
    /// Trials that ended `http_error` (mid-trial transport error).
    /// Surfaced as a coverage record — like `rate_limited`, never a
    /// fault and never attributed to the agent or the tool.
    pub http_error: usize,
    /// Cells the budget skipped before they could start.
    pub absent: usize,
    /// Pass rate over trials that ran; `None` when nothing ran.
    pub pass_rate: Option<f64>,
}

/// One scenario's A/B result: per-arm summaries, the delta, and the
/// underlying rows verbatim (the report is auditable, not a black box).
#[derive(Debug, Clone, Serialize)]
pub struct AblationReport {
    /// Report shape version.
    pub ablation_version: u32,
    /// The scenario name.
    pub scenario: String,
    /// The full-arm (AIX artifacts present) summary.
    pub full: ArmSummary,
    /// The ablated-arm (raw binaries) summary.
    pub ablated: ArmSummary,
    /// `pass_rate(full) − pass_rate(ablated)` — the measured value of
    /// the AIX investment for this scenario. `None` when either arm
    /// has no runnable trials; undefined is never silently zero.
    pub delta: Option<f64>,
    /// The full arm's rotation rows, verbatim.
    pub full_rows: Vec<RotationEvent>,
    /// The ablated arm's rotation rows, verbatim.
    pub ablated_rows: Vec<RotationEvent>,
}

/// Run the A/B ablation for one scenario: ≥3 runs per arm (the evals.md
/// Step 5 floor, enforced loudly), the full arm first, then the
/// ablated arm, over the same model list and repetitions. The budget
/// closure is shared across both arms, so a mid-ablation expiry lands
/// as absent cells in whichever arm runs short — recorded, never
/// failed, never dropped.
///
/// The artifact paths must not collide with the scenario's own fixture
/// files or distractors: an artifact must never overwrite scenario
/// material the checks depend on (e.g. a corrupt config is the fault
/// injection). Collisions are a loud error, not a silent overwrite.
pub fn run_ablation(
    scenario: &Scenario,
    models: &[String],
    reps: u32,
    artifacts: &AixArtifacts,
    factories: AblationFactories<'_>,
    bounds: &Bounds,
    budget_remaining: &dyn Fn() -> bool,
) -> Result<AblationReport, String> {
    if reps < REPS_FLOOR {
        return Err(format!(
            "ablation needs ≥{REPS_FLOOR} runs per arm to support a delta claim (got {reps})"
        ));
    }
    let artifact_files = artifacts.fixture_files();
    for (path, _) in &artifact_files {
        let collides = scenario
            .fixture_files
            .iter()
            .map(|(p, _)| p.as_str())
            .chain(scenario.distractors.iter().map(|d| d.path.as_str()))
            .any(|p| p == path);
        if collides {
            return Err(format!(
                "AIX artifact '{path}' collides with scenario material for '{}' — refusing to overwrite",
                scenario.name
            ));
        }
    }

    let full_rows = run_rotation_with_arm(
        std::slice::from_ref(scenario),
        models,
        reps,
        &ArmContext {
            arm: Arm::Full,
            extra_fixture_files: &artifact_files,
        },
        factories.full,
        bounds,
        budget_remaining,
    )?;
    let ablated_rows = run_rotation_with_arm(
        std::slice::from_ref(scenario),
        models,
        reps,
        &ArmContext {
            arm: Arm::Ablated,
            extra_fixture_files: &[],
        },
        factories.ablated,
        bounds,
        budget_remaining,
    )?;

    let full = arm_summary(Arm::Full, &full_rows);
    let ablated = arm_summary(Arm::Ablated, &ablated_rows);
    let delta = match (full.pass_rate, ablated.pass_rate) {
        (Some(full_rate), Some(ablated_rate)) => Some(full_rate - ablated_rate),
        _ => None,
    };
    Ok(AblationReport {
        ablation_version: ABLATION_VERSION,
        scenario: scenario.name.clone(),
        full,
        ablated,
        delta,
        full_rows,
        ablated_rows,
    })
}

/// Summarize one arm's rotation rows into an [`ArmSummary`].
fn arm_summary(arm: Arm, rows: &[RotationEvent]) -> ArmSummary {
    let mut summary = ArmSummary {
        arm,
        trials: 0,
        passed: 0,
        failed: 0,
        invalid_output: 0,
        rate_limited: 0,
        http_error: 0,
        absent: 0,
        pass_rate: None,
    };
    for row in rows {
        match row {
            RotationEvent::Trial { report } => {
                summary.trials += 1;
                match report.status {
                    TrialStatus::Passed => summary.passed += 1,
                    TrialStatus::Failed => summary.failed += 1,
                    TrialStatus::InvalidOutput => summary.invalid_output += 1,
                    TrialStatus::RateLimited => summary.rate_limited += 1,
                    TrialStatus::HttpError => summary.http_error += 1,
                }
            }
            RotationEvent::Absent { .. } => summary.absent += 1,
        }
    }
    let ran = summary.passed + summary.failed + summary.invalid_output;
    if ran > 0 {
        summary.pass_rate = Some(summary.passed as f64 / ran as f64);
    }
    summary
}
