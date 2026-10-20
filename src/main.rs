//! Purpose: CLI entry point of the evallerina harness.
//! Responsibilities: expose the tier commands the justfile recipes
//! invoke — `smoke` (run the replay smoke scenario, exit code carries
//! pass/fail) and `live` (tier-2 live rotation over scenario × model
//! cells via the OpenRouter :free action protocol).
//! Rationale: tier discipline — the binary gates tier-0/1 locally and
//! in CI; tier-2 never gates a push. `live` prints one JSON event per
//! cell (trial row or absent record) and exits nonzero only when a
//! trial actually failed or malformed — skipped cells are the record.

use std::io::Write;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use evallerina::ablation::{
    AblationFactories, AixArtifacts, AixSource, ToolBinaries, run_ablation,
};
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
    /// A/B ablation over one scenario (evals.md Step 5): the full arm
    /// (AIX artifacts provisioned) vs the ablated arm (raw binaries),
    /// ≥3 runs per arm over the configured model registry. Writes the
    /// per-arm rotation rows as ArmReport JSONL to --out and prints the
    /// per-arm summary + delta as JSON on stdout.
    Ablate {
        /// Scenario name (a live battery scenario; unknown names are a
        /// loud error).
        scenario: String,
        /// Repetitions per arm (the evals.md Step 5 delta floor is 3).
        #[arg(long, default_value_t = DEFAULT_REPETITIONS)]
        reps: u32,
        /// Run only this model id (default: the whole registry, in order).
        #[arg(long, default_value = None)]
        model: Option<String>,
        /// Dry-run: every cell is recorded absent — no model calls, no
        /// network; exercises the full report path hermetically.
        #[arg(long)]
        dry_run: bool,
        /// Real artifact generation (opt-in; evallerina-0j8): invoke the
        /// provisioned `wai` binary and harvest what it produces for the
        /// full arm, instead of the static hermetic contents. A tool
        /// failure is a loud error, never a placeholder fallback.
        #[arg(long)]
        real_artifacts: bool,
        /// Per-run wall-clock budget, minutes; cells not started before
        /// it expires are recorded absent, never failed.
        #[arg(long, default_value_t = DEFAULT_BUDGET_MINS)]
        budget_mins: u64,
        /// Path the ArmReport JSONL rows are written to (full arm rows
        /// first, then ablated, in run order).
        #[arg(long)]
        out: String,
    },
    /// Aggregate report rows (one JSON value per line in a JSONL file —
    /// tier-2 rotation events or tier-1 rows) into the per-channel
    /// fault dashboard. Prints the dashboard JSON to stdout; exits
    /// nonzero on aggregation errors (unknown scenario, version
    /// mismatch), never on fault findings.
    Report {
        /// Path to the JSONL report rows to aggregate.
        path: String,
        /// Run the fault-routing step after aggregation (dry-run by
        /// default): attach the additive `routed` summary to the
        /// dashboard and write proposed-tickets.jsonl next to the report.
        #[arg(long)]
        route: bool,
        /// File the proposed tickets in their mapped repositories
        /// (explicit cross-repo write authorization, Phase 2).
        /// Requires --route.
        #[arg(long)]
        file: bool,
    },
    /// Tier-0 routing-map parity gate (evallerina-0sy): diff
    /// routing.toml against dulce-de-leche's MANAGED-TOOLS block
    /// (../dulce-de-leche/README.md). Prints the parity report as JSON
    /// (degraded flag + one row per managed tool) and exits nonzero iff
    /// any row failed; an absent ddl sibling degrades to a warning row
    /// with exit 0 (standalone clones), never covering a parse failure.
    RoutingParity,
}

/// The hermetic static artifact set the full arm provisions by default
/// (the ablated-arm baseline stays raw binaries either way). Real
/// generation — the provisioned `wai` binary harvesting the actual
/// managed AGENTS.md, `.genesis/tools.toml` and `llms.txt` — is opt-in
/// via --real-artifacts (evallerina-0j8).
const DEFAULT_AIX_ARTIFACTS: AixArtifacts = AixArtifacts {
    llms_txt: "llms.txt content: wai status --json",
    managed_agents_md: "<!-- WAI:START -->managed guidance<!-- WAI:END -->",
    tools_toml: "[tools]\nwai = { cmd = \"wai\" }\n",
};

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
        Command::Report { path, route, file } => match run_report(&path, route, file) {
            Ok(code) => code,
            Err(msg) => {
                eprintln!("{msg}");
                ExitCode::FAILURE
            }
        },
        Command::RoutingParity => match run_routing_parity() {
            Ok(code) => code,
            Err(msg) => {
                eprintln!("{msg}");
                ExitCode::FAILURE
            }
        },
        Command::Ablate {
            scenario,
            reps,
            model,
            dry_run,
            real_artifacts,
            budget_mins,
            out,
        } => match run_ablate(
            &scenario,
            reps,
            model,
            dry_run,
            real_artifacts,
            budget_mins,
            &out,
        ) {
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

/// Tier-0 routing-map parity gate (evallerina-0sy): read the checked-in
/// routing.toml and the ddl sibling README relative to the repo root
/// (discovered worktree-safe), run
/// [`evallerina::routing_parity::routing_parity`], and print the parity
/// report as JSON. Exit nonzero
/// iff any parity row failed (`ERR_ROUTING_PARITY` rows carry tool +
/// detail); an absent ddl sibling degrades to a warning row with exit
/// 0 — parse failures stay loud and nonzero.
fn run_routing_parity() -> Result<ExitCode, String> {
    let root = evallerina::routing::discover_root()?;
    let root_path = std::path::PathBuf::from(&root);
    let toml_path = root_path.join("routing.toml");
    let toml = std::fs::read_to_string(&toml_path)
        .map_err(|e| format!("cannot read {}: {e}", toml_path.display()))?;
    let readme_path = root_path.join(evallerina::routing_parity::DDL_README);
    let readme = std::fs::read_to_string(&readme_path).ok();
    let report = evallerina::routing_parity::routing_parity(readme.as_deref(), &toml, &root_path)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report.to_json()).expect("parity report serializes")
    );
    if report.failed() {
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
fn run_report(path: &str, route: bool, file: bool) -> Result<ExitCode, String> {
    evallerina::report::validate_route_flags(route, file)?;
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read report rows from {path}: {e}"))?;
    let rows: Vec<serde_json::Value> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| format!("malformed report row: {e}")))
        .collect::<Result<Vec<_>, String>>()?;
    let configured: Vec<String> = ordered_ids().into_iter().map(String::from).collect();
    let mut dashboard = evallerina::report::aggregate_rows(
        &rows,
        &configured,
        &evallerina::report::scenario_channel,
    )?;
    if route {
        let mut runner = evallerina::routing::run_bd;
        evallerina::report::run_routing_step(
            &mut dashboard,
            &rows,
            &configured,
            std::path::Path::new(path),
            file,
            &mut runner,
        )?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&dashboard).expect("dashboard serializes")
    );
    Ok(ExitCode::SUCCESS)
}

/// A/B ablation CLI handler (evallerina-wdx): resolve the named
/// scenario and model list, run both arms via
/// [`evallerina::ablation::run_ablation`], write the ArmReport JSONL
/// rows (full arm first, then ablated, in run order), and print the
/// per-arm summary + delta as JSON on stdout. Measurement, not a gate:
/// the exit code is success whenever the ablation ran; errors — unknown
/// scenario, artifact-path collision, generation failure, unwritable
/// output — are loud and nonzero, never silent.
fn run_ablate(
    scenario: &str,
    reps: u32,
    model: Option<String>,
    dry_run: bool,
    real_artifacts: bool,
    budget_mins: u64,
    out: &str,
) -> Result<ExitCode, String> {
    let scenario = evallerina::scenario::live_scenario_by_name(scenario)?;
    let models: Vec<String> = match model {
        Some(id) => vec![id],
        None => ordered_ids().into_iter().map(String::from).collect(),
    };
    let artifacts: AixSource<'static> = if real_artifacts {
        let wai = ToolBinaries::discover_wai().ok_or_else(|| {
            "real artifact generation requested but no `wai` binary is on PATH — refusing to fake it"
                .to_owned()
        })?;
        AixSource::Real(ToolBinaries { wai })
    } else {
        AixSource::Static(&DEFAULT_AIX_ARTIFACTS)
    };
    // Dry-run: transports are constructed but never called — the zero
    // budget records every cell absent before any trial starts. No
    // key required, no network. Non-dry-run: the real tier-2
    // transport, which refuses to start without OPENROUTER_API_KEY.
    let report = if dry_run {
        let mut full_factory = |m: &str| {
            Ok(Box::new(OpenRouterTransport::new(m, "dry-run-no-network"))
                as Box<dyn evallerina::live::Transport>)
        };
        let mut ablated_factory = |m: &str| {
            Ok(Box::new(OpenRouterTransport::new(m, "dry-run-no-network"))
                as Box<dyn evallerina::live::Transport>)
        };
        run_ablation(
            &scenario,
            &models,
            reps,
            artifacts,
            AblationFactories {
                full: &mut full_factory,
                ablated: &mut ablated_factory,
            },
            &evallerina::live::Bounds::default(),
            &|| false,
        )
    } else {
        let start = Instant::now();
        let budget = Duration::from_secs(budget_mins * 60);
        let mut full_factory = |m: &str| {
            OpenRouterTransport::from_env(m)
                .map(|t| Box::new(t) as Box<dyn evallerina::live::Transport>)
        };
        let mut ablated_factory = |m: &str| {
            OpenRouterTransport::from_env(m)
                .map(|t| Box::new(t) as Box<dyn evallerina::live::Transport>)
        };
        run_ablation(
            &scenario,
            &models,
            reps,
            artifacts,
            AblationFactories {
                full: &mut full_factory,
                ablated: &mut ablated_factory,
            },
            &evallerina::live::Bounds::default(),
            &|| start.elapsed() < budget,
        )
    }?;

    let mut rows = std::fs::File::create(out)
        .map_err(|e| format!("cannot write report rows to {out}: {e}"))?;
    for event in report.full_rows.iter().chain(&report.ablated_rows) {
        writeln!(
            rows,
            "{}",
            serde_json::to_string(event).expect("row serializes")
        )
        .map_err(|e| format!("cannot write report rows to {out}: {e}"))?;
    }
    let summary = serde_json::json!({
        "ablation_version": report.ablation_version,
        "scenario": report.scenario,
        "full": report.full,
        "ablated": report.ablated,
        "delta": report.delta,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("summary serializes")
    );
    Ok(ExitCode::SUCCESS)
}
