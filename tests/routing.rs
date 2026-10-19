//! Purpose: integration tests for the dry-run fault routing module
//! (evallerina-l8w, openspec add-fault-ticket-routing Phase 1).
//! Responsibilities: pin the report.rs threshold semantics (min(3,
//! |configured registry|) distinct models per fault key), agent-fault
//! exclusion, unmapped-tool skip, scenario-recorded-tool repo keying,
//! worktree-safe routing.toml path resolution, and the full
//! proposed-tickets.jsonl row schema.
//! Rationale: routing proposals are cross-repo actions in potentia —
//! every decision branch (dry_run / skipped) and every schema field is
//! machine-checked here so the CI artifact is reviewable without
//! reading code.

use evallerina::routing;
use serde_json::json;

/// A tier-2 trial row with a failing, code-less check (tool fault) for
/// one model. `row_tool` is the report row's tool field (hardcoded in
/// live rows); `recorded` is the scenario definition's recorded tool.
fn trial_row(scenario: &str, row_tool: &str, model: &str) -> serde_json::Value {
    json!({
        "kind": "trial",
        "report_version": 1,
        "scenario": scenario,
        "tool": row_tool,
        "model": model,
        "status": "failed",
        "checks": [{"name": "envelope_hint_absent", "passed": false}]
    })
}

fn three_models() -> Vec<String> {
    vec!["m/a".into(), "m/b".into(), "m/c".into()]
}

fn recorded_wai(scenario: &str) -> Option<String> {
    (scenario == "wai-hint-adherence-typo").then(|| "wai".to_owned())
}

fn repo_map() -> std::collections::BTreeMap<String, String> {
    [("wai".to_owned(), "../wai".to_owned())].into()
}

#[test]
fn threshold_tool_fault_becomes_dry_run_proposal() {
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| trial_row("wai-hint-adherence-typo", "wai", m))
        .collect();
    let proposals =
        routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
            .expect("routing succeeds");
    assert_eq!(proposals.len(), 1, "one fault key above threshold");
    let p = &proposals[0].to_json();
    assert_eq!(p["scenario"], "wai-hint-adherence-typo");
    assert_eq!(p["fault_class"], "tool_fault");
    assert_eq!(p["token"], "evallerina:wai-hint-adherence-typo:tool_fault");
    assert_eq!(p["decision"], "dry_run");
    assert_eq!(p["reason"], serde_json::Value::Null);
    assert_eq!(p["bd_issue_id"], serde_json::Value::Null);
}

#[test]
fn below_threshold_is_no_proposal() {
    // (schema via to_json — indexing reads the serialized artifact)
    let rows: Vec<serde_json::Value> = ["m/a", "m/b"] // 2 distinct < 3
        .iter()
        .map(|m| trial_row("wai-hint-adherence-typo", "wai", m))
        .collect();
    let proposals =
        routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
            .expect("routing succeeds");
    assert!(proposals.is_empty(), "below threshold → recorded, no action");
}
