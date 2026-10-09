//! Purpose: CLI entry point of the evallerina harness.
//! Responsibilities: expose the tier commands the justfile recipes
//! invoke — `smoke` (run the replay smoke scenario, exit code carries
//! pass/fail) and `live` (tier-2 live rotation over scenario × model
//! cells via the OpenRouter :free action protocol).
//! Rationale: tier discipline — the binary gates tier-0/1 locally and
//! in CI; tier-2 never gates a push. `live` prints one JSON event per
//! cell (trial row or absent record) and exits nonzero only when a
//! trial actually failed or malformed — skipped cells are the record.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use evallerina::live::{OpenRouterTransport, RotationEvent, TrialStatus, run_rotation};
use evallerina::recorded::RecordedTrajectory;
use evallerina::registry::{DEFAULT_REPETITIONS, ordered_ids};
use evallerina::scenario::{SCENARIOS_DIR, live_scenario_by_name, live_scenarios, smoke_scenario};

/// Default per-run wall-clock budget for the live rotation, minutes.
const DEFAULT_BUDGET_MINS: u64 = 20;

/// Evals with an avatar — measuring whether agents actually use the tools.
#[derive(Parser)]
#[command(name = "evallerina", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the tier-1 smoke replay (no model, no network). Exit 0 iff green.
    Smoke,
    /// Tier-2 live rotation (requires OPENROUTER_API_KEY). One JSON
    /// event per cell on stdout: a report_version 1 trial row, or an
    /// absent record when the per-run budget skipped the cell.
    Live {
        /// Optional scenario name filter (default: the whole live battery).
        #[arg(default_value = None)]
        scenario: Option<String>,
        /// Repetitions per scenario × model cell.
        #[arg(long, default_value_t = DEFAULT_REPETITIONS)]
        reps: u32,
        /// Run only this model id (default: the whole registry, in order).
        #[arg(long, default_value = None)]
        model: Option<String>,
        /// Per-run wall-clock budget, minutes; cells not started before
        /// it expires are recorded absent, never failed.
        #[arg(long, default_value_t = DEFAULT_BUDGET_MINS)]
        budget_mins: u64,
    },
    /// Aggregate report rows (one JSON value per line in a JSONL file —
    /// tier-2 rotation events or tier-1 rows) into the per-channel
    /// fault dashboard. Prints the dashboard JSON to stdout; exits
    /// nonzero on aggregation errors (unknown scenario, version
    /// mismatch), never on fault findings.
    Report {
        /// Path to the JSONL report rows to aggregate.
        path: String,
    },
}

fn main() -> std::process::ExitCode {
    match Cli::parse().command {
        Command::Smoke => {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(SCENARIOS_DIR)
                .join("wai-hint-adherence-smoke.json");
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let trajectory = RecordedTrajectory::from_json(&content)
                .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
            let report = smoke_scenario()
                .run(trajectory.steps())
                .unwrap_or_else(|e| panic!("replay failed: {e}"));
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            if report.passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Command::Live {
            scenario,
            reps,
            model,
            budget_mins,
        } => match run_live(scenario, reps, model, budget_mins) {
            Ok(code) => code,
            Err(msg) => {
                eprintln!("{msg}");
                ExitCode::FAILURE
            }
        },
        Command::Report { path } => match run_report(&path) {
            Ok(code) => code,
            Err(msg) => {
                eprintln!("{msg}");
                ExitCode::FAILURE
            }
        },
    }
}

/// Tier-2 live rotation: the scenario battery (or one named scenario)
/// × selected models × reps, in registry order, under a per-run
/// wall-clock budget. One JSON event per cell on stdout — a
/// report_version 1 trial row or an absent record. Exit nonzero iff a
/// trial failed or malformed; rate-limited and budget-skipped cells are
/// recorded in their rows, they are not failures (the row is the
/// record).
fn run_live(
    scenario: Option<String>,
    reps: u32,
    model: Option<String>,
    budget_mins: u64,
) -> Result<ExitCode, String> {
    let scenarios: Vec<_> = match scenario {
        Some(name) => vec![live_scenario_by_name(&name)?],
        None => live_scenarios(),
    };
    let models: Vec<String> = match model {
        Some(id) => vec![id],
        None => ordered_ids().into_iter().map(String::from).collect(),
    };
    let start = Instant::now();
    let budget = Duration::from_secs(budget_mins * 60);
    let mut factory = |m: &str| {
        OpenRouterTransport::from_env(m)
            .map(|t| Box::new(t) as Box<dyn evallerina::live::Transport>)
    };
    let events = run_rotation(
        &scenarios,
        &models,
        reps,
        &mut factory,
        &evallerina::live::Bounds::default(),
        &|| start.elapsed() < budget,
    )?;
    let mut trial_failed = false;
    for event in &events {
        if let RotationEvent::Trial { report } = event {
            trial_failed |= matches!(
                report.status,
                TrialStatus::Failed | TrialStatus::InvalidOutput
            );
        }
        println!("{}", serde_json::to_string(event).unwrap());
    }
    if trial_failed {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Aggregate a JSONL file of report rows into the fault dashboard.
/// The configured model registry comes from the checked-in registry;
/// the channel map from the live scenario registry. Findings do not
/// affect the exit code — the dashboard is the record, and nonzero
/// exits are reserved for aggregation errors.
fn run_report(path: &str) -> Result<ExitCode, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read report rows from {path}: {e}"))?;
    let rows: Vec<serde_json::Value> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| format!("malformed report row: {e}")))
        .collect::<Result<Vec<_>, String>>()?;
    let configured: Vec<String> = ordered_ids().into_iter().map(String::from).collect();
    let dashboard = evallerina::report::aggregate_rows(
        &rows,
        &configured,
        &evallerina::report::scenario_channel,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&dashboard).expect("dashboard serializes")
    );
    Ok(ExitCode::SUCCESS)
}
