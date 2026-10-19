//! Purpose: End-to-end CLI contract tests for the `evallerina ablate`
//! subcommand (evallerina-wdx, evals.md Step 5).
//! Responsibilities: pin the CLI surface — `ablate --help` exits 0 and
//! names the scenario arg — and the hermetic dry-run path: a temp
//! sandbox run with zero budget writes the ArmReport JSONL rows and the
//! summary without any model call or network, and bad paths fail
//! loudly, never silently.
//! Rationale: the ablation CLI is the production entry to the A/B
//! harness; its contract (no silent degradation, rows are the record)
//! must hold end to end through the binary, not just through the
//! library API.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_evallerina"))
}

/// A fresh unique temp sandbox dir; removed after use.
fn sandbox(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "evallerina-ablate-cli-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
    ));
    std::fs::create_dir_all(&dir).expect("create sandbox");
    dir
}

/// Step 1 (evallerina-wdx): `evallerina ablate --help` exits 0 and
/// names the positional scenario arg.
#[test]
fn ablate_help_exits_zero_and_names_scenario_arg() {
    let out = bin().args(["ablate", "--help"]).output().expect("run help");
    assert!(out.status.success(), "ablate --help exits 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("scenario"),
        "help names the scenario arg: {stdout}"
    );
}
