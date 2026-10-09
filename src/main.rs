//! Purpose: CLI entry point of the evallerina harness.
//! Responsibilities: expose the tier commands the justfile recipes
//! invoke — `smoke` (run the replay smoke scenario, exit code carries
//! pass/fail) and `live` (tier-2 live model runs; stub until
//! evallerina-1vw lands the OpenRouter action-protocol runner).
//! Rationale: tier discipline — the binary gates tier-0/1 locally and
//! in CI; tier-2 never gates a push and is a deliberately separate
//! ticket, so `live` reports unimplemented rather than pretending.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use evallerina::recorded::RecordedTrajectory;
use evallerina::scenario::{SCENARIOS_DIR, smoke_scenario};

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
    /// Tier-2 live model run (requires OPENROUTER_API_KEY). Not yet implemented.
    Live {
        /// Optional scenario name filter.
        #[arg(default_value = None)]
        scenario: Option<String>,
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
        Command::Live { .. } => {
            eprintln!(
                "live runs are not implemented yet — tier-2 runner is evallerina-1vw; \
                 use `just tier1` / `just smoke` for tier-1 replay"
            );
            ExitCode::FAILURE
        }
    }
}
