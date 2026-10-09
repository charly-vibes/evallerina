//! Purpose: Recorded agent trajectories — the tier-1 replay input.
//! Responsibilities: define the on-disk trajectory schema under
//! `scenarios/*.json` (name, provenance, tool/version recorded against,
//! ordered AgentStep-like steps) and convert it into
//! `genesis::evals::AgentStep` for `Scenario::run`.
//! Rationale: recordings are captured from real tool invocations (see
//! each file's `provenance` field) and committed so tier-1 replay is
//! deterministic and offline. `executed` is always true for genuine
//! captures; recordings keep it explicit so doctored replays used in
//! detection tests can model hallucination honestly.

use genesis::evals::AgentStep;
use serde::Deserialize;

/// One recorded tool invocation.
#[derive(Debug, Clone, Deserialize)]
pub struct RecordedStep {
    /// Command line as issued by the agent.
    pub command: String,
    /// Captured stdout of the invocation.
    pub stdout: String,
    /// Captured stderr of the invocation.
    pub stderr: String,
    /// Exit code of the invocation.
    pub exit_code: i32,
    /// Whether a subprocess call was actually observed.
    pub executed: bool,
}

/// When and against what the trajectory was captured.
#[derive(Debug, Clone, Deserialize)]
pub struct RecordedMeta {
    /// Tool under evaluation (e.g. `wai`).
    pub tool: String,
    /// Raw tool version at capture time, verbatim.
    pub version: String,
    /// Capture date (`YYYY-MM-DD`).
    pub date: String,
}

/// A complete recorded trajectory file under `scenarios/`.
#[derive(Debug, Clone, Deserialize)]
pub struct RecordedTrajectory {
    /// Stable scenario name; must match the file name.
    pub name: String,
    /// Where the recording came from: ticket, corpus note, or live
    /// capture recipe.
    pub provenance: String,
    /// Capture metadata.
    pub recorded: RecordedMeta,
    /// Ordered steps, as issued.
    pub steps: Vec<RecordedStep>,
}

impl RecordedTrajectory {
    /// Parse a trajectory from its JSON file content.
    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    /// Convert into replay-ready `AgentStep`s for `Scenario::run`.
    pub fn steps(&self) -> Vec<AgentStep> {
        self.steps
            .iter()
            .map(|s| AgentStep {
                command: s.command.clone(),
                stdout: s.stdout.clone(),
                stderr: s.stderr.clone(),
                exit_code: s.exit_code,
                executed: s.executed,
            })
            .collect()
    }
}
