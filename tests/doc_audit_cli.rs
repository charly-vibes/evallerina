//! Purpose: end-to-end CLI contract tests for `evallerina doc-audit`
//! routing intake (evallerina-e4x, openspec add-ecosystem-doc-evals
//! tasks 3.1–3.2).
//! Responsibilities: pin the dry-run default through the binary — the
//! doc-audit routing step emits proposal rows and never files without
//! explicit `--file` — plus the `--proposals-out` JSONL artifact and
//! the non-fatal bd-failure path under `--file`.
//! Rationale: the filing path can create real tickets in real repos;
//! its anti-goals (no filing without `--file`, dry-run default) must
//! hold end to end through the binary, not just through the library
//! API. Every test runs in an isolated git sandbox with a fixture
//! binary — the trackers of evallerina and every family repo are
//! never touched.

use serde_json::Value;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_evallerina"))
}

/// Seed a hermetic doc-audit sandbox: a git repo (so `doc-audit`'s
/// worktree-safe root discovery resolves here) containing a fixture
/// binary whose `--help` exposes `--some-flag`, a README that never
/// mentions it (→ `ERR_FEATURE_UNDOCUMENTED`), and the two checked-in
/// config shapes mapping the tool to this same sandbox repo.
fn seed_sandbox(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "evallerina-doc-audit-cli-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
    ));
    std::fs::create_dir_all(&dir).expect("create sandbox");
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .output()
        .expect("git init in sandbox");
    assert!(git.status.success(), "git init failed in sandbox");

    // Fixture binary: `--help` harvests exactly one feature token.
    let fixtool = dir.join("fixtool");
    std::fs::write(
        &fixtool,
        "#!/bin/sh\necho \"Usage: fixtool [OPTIONS]\"\necho \"  --some-flag  do the thing\"\n",
    )
    .expect("write fixtool");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fixtool, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fixtool");
    }

    // Docs never document the flag → the audit row exists.
    std::fs::write(dir.join("README.md"), "# fixtool\n\nRuns the thing.\n").expect("write README");

    std::fs::write(
        dir.join("doc_audit.toml"),
        "[fixtool]\nrepo = \".\"\nbinary = \"fixtool\"\n",
    )
    .expect("write doc_audit.toml");
    std::fs::write(dir.join("routing.toml"), "[fixtool]\nrepo = \".\"\n")
        .expect("write routing.toml");
    dir
}

/// Run `doc-audit` inside the sandbox with the fixture binary on PATH.
fn run_doc_audit(sb: &std::path::Path, extra: &[&str]) -> (std::process::Output, Value) {
    // Fixture binary on PATH, prepended so git/system tools stay
    // reachable (doc-audit's root discovery shells out to git).
    let path = std::env::var("PATH").unwrap_or_default();
    let out = bin()
        .args(["doc-audit"])
        .args(extra)
        .current_dir(sb)
        .env("PATH", format!("{}:{}", sb.display(), path))
        .output()
        .expect("run doc-audit");
    let json: Value = serde_json::from_slice(&out.stdout).expect("doc-audit prints JSON");
    (out, json)
}

/// The one expected audit finding in the sandbox: `--some-flag` is
/// undocumented, so it must appear as an audit row and route.
fn undocumented_someflag(json: &Value) {
    let rows = json["audit_rows"].as_array().expect("audit_rows array");
    assert!(
        rows.iter().any(|r| {
            r["tool"] == "fixtool"
                && r["token"] == "someflag"
                && r["code"] == "ERR_FEATURE_UNDOCUMENTED"
        }),
        "expected the someflag ERR_FEATURE_UNDOCUMENTED row, got: {rows:?}"
    );
}

#[test]
fn doc_audit_routes_dry_run_by_default_and_never_files() {
    let sb = seed_sandbox("dry-run-default");
    let jsonl_path = sb.join("proposed-tickets.jsonl");
    let (out, json) = run_doc_audit(&sb, &["--proposals-out", jsonl_path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "doc-audit exits zero on findings (the audit is the record): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    undocumented_someflag(&json);

    // The routing intake is part of the output: one dry-run proposal
    // keyed on the audited tool, with the fingerprint token.
    let proposals = json["proposals"].as_array().expect("proposals array");
    assert_eq!(proposals.len(), 1, "one fault key above threshold");
    let p = &proposals[0];
    assert_eq!(p["decision"], "dry_run");
    assert_eq!(p["tool"], "fixtool");
    assert_eq!(p["token"], "evallerina:doc-audit:ERR_FEATURE_UNDOCUMENTED");
    assert_eq!(p["bd_issue_id"], Value::Null, "dry-run files nothing");

    // Dry-run default anti-goal: without --file no bd call ever
    // happens — the sandbox tracker is untouched.
    assert!(!sb.join(".beads").exists(), "no filing without --file");

    // The JSONL artifact is the proposed-tickets.jsonl schema: one
    // machine-checkable object per line.
    let jsonl = std::fs::read_to_string(&jsonl_path).expect("proposals JSONL written");
    let lines: Vec<&str> = jsonl.lines().collect();
    assert_eq!(lines.len(), 1, "one object per line");
    let row: Value = serde_json::from_str(lines[0]).expect("each line is a JSON object");
    assert_eq!(
        row["token"],
        "evallerina:doc-audit:ERR_FEATURE_UNDOCUMENTED"
    );
}

#[test]
fn doc_audit_file_flag_is_the_only_escalation_and_bd_failure_is_not_fatal() {
    let sb = seed_sandbox("file-escalation");
    let (out, json) = run_doc_audit(&sb, &["--file"]);
    // Filing is an optimization over the report: a bd failure in the
    // target repo is the record (skipped/bd_failure), never an error.
    assert!(
        out.status.success(),
        "bd failure must not crash doc-audit: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let p = &json["proposals"].as_array().expect("proposals array")[0];
    assert_eq!(p["decision"], "skipped");
    assert_eq!(p["reason"], "bd_failure");
    assert_eq!(p["bd_issue_id"], Value::Null);
}
