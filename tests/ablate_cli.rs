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

use serde_json::Value;

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

/// Step 2 (evallerina-wdx): end-to-end dry-run in a temp sandbox — no
/// network, no model call, no OPENROUTER_API_KEY — writes the ArmReport
/// JSONL rows and prints the summary. With a zero budget every cell is
/// recorded absent (registry × reps × 2 arms), the rows carry the arm
/// field, and the delta stays null (undefined is never zero).
#[test]
fn ablate_dry_run_writes_report_files_in_temp_sandbox() {
    let dir = sandbox("dry-run");
    let rows_path = dir.join("rows.jsonl");
    let out = bin()
        .args([
            "ablate",
            "wai-hint-adherence-smoke",
            "--dry-run",
            "--out",
            rows_path.to_str().unwrap(),
        ])
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("run ablate dry-run");
    assert!(
        out.status.success(),
        "dry-run succeeds without a key or network: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Summary on stdout: per-arm summaries + delta, report_version 1.
    let summary: Value = serde_json::from_slice(&out.stdout).expect("summary JSON on stdout");
    assert_eq!(summary["ablation_version"], 1);
    assert_eq!(summary["scenario"], "wai-hint-adherence-smoke");
    assert_eq!(summary["full"]["arm"], "full");
    assert_eq!(summary["ablated"]["arm"], "ablated");
    assert_eq!(summary["delta"], Value::Null, "no trials → undefined delta");

    // ArmReport JSONL rows: every registry model × 3 reps × 2 arms,
    // each recorded absent with its arm — never failed, never dropped.
    let jsonl = std::fs::read_to_string(&rows_path).expect("rows.jsonl written");
    let rows: Vec<Value> = jsonl
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("row parses"))
        .collect();
    assert_eq!(rows.len(), 24, "4 registry models × 3 reps × 2 arms");
    let full_count = rows
        .iter()
        .filter(|r| r["kind"] == "absent" && r["arm"] == "full")
        .count();
    let ablated_count = rows
        .iter()
        .filter(|r| r["kind"] == "absent" && r["arm"] == "ablated")
        .count();
    assert_eq!(full_count, 12);
    assert_eq!(ablated_count, 12);
    assert_eq!(summary["full"]["absent"], 12);
    assert_eq!(summary["ablated"]["absent"], 12);

    std::fs::remove_dir_all(&dir).ok();
}

/// Loud errors on bad paths, never silent: an unknown scenario name
/// fails with a nonzero exit naming the problem.
#[test]
fn ablate_unknown_scenario_fails_loudly() {
    let dir = sandbox("bad-scenario");
    let out = bin()
        .args([
            "ablate",
            "no-such-scenario",
            "--dry-run",
            "--out",
            dir.join("rows.jsonl").to_str().unwrap(),
        ])
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("run ablate");
    assert!(!out.status.success(), "unknown scenario fails");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown scenario"),
        "stderr names the problem: {stderr}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Loud errors on bad paths, never silent: an unwritable --out path
/// fails with a nonzero exit, after nothing is silently dropped.
#[test]
fn ablate_unwritable_out_path_fails_loudly() {
    let dir = sandbox("bad-out");
    let out = bin()
        .args([
            "ablate",
            "wai-hint-adherence-smoke",
            "--dry-run",
            "--out",
            dir.join("no-such-dir").join("rows.jsonl").to_str().unwrap(),
        ])
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("run ablate");
    assert!(!out.status.success(), "unwritable out path fails");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot write report rows"),
        "stderr names the problem: {stderr}"
    );
    std::fs::remove_dir_all(&dir).ok();
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
