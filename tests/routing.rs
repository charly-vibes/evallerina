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

#[test]
fn agent_faults_are_never_routed() {
    // Failing checks WITH an ERR_* code are agent faults: they inform
    // AIX-channel work in genesis (a human decision), never tickets.
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| {
            let mut r = trial_row("wai-hint-adherence-typo", "wai", m);
            r["checks"] = json!([{"name": "hint_followed", "passed": false,
                                   "code": "ERR_ENVELOPE_HINT_BLINDNESS"}]);
            r
        })
        .collect();
    let proposals =
        routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
            .expect("routing succeeds");
    assert!(proposals.is_empty(), "agent faults must not produce proposals");
}

#[test]
fn unmapped_tool_is_skipped_not_fatal() {
    // Above threshold, but routing.toml has no [wai] entry here.
    let empty_map = std::collections::BTreeMap::new();
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| trial_row("wai-hint-adherence-typo", "wai", m))
        .collect();
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &empty_map)
        .expect("routing must not fail on an unmapped tool");
    assert_eq!(proposals.len(), 1, "skipped is still recorded, not dropped");
    let p = &proposals[0].to_json();
    assert_eq!(p["decision"], "skipped");
    assert_eq!(p["reason"], "unmapped_tool");
    assert_eq!(p["repo"], serde_json::Value::Null);
}

#[test]
fn repo_keys_on_recorded_tool_not_row_tool() {
    // Live rows hardcode tool: "wai" (src/live.rs); a drifted row tool
    // (the hhs bug class) must never send the ticket to the wrong repo.
    // routing.toml here maps "dont" only — the scenario's recorded tool.
    let mut drifted_map = std::collections::BTreeMap::new();
    drifted_map.insert("dont".to_owned(), "../dont".to_owned());
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| {
            // Row tool says "wai"; scenario definition recorded "dont".
            trial_row("dont-hint-adherence-lifecycle", "wai", m)
        })
        .collect();
    let recorded_dont = |s: &str| (s == "dont-hint-adherence-lifecycle").then(|| "dont".to_owned());
    let proposals = routing::propose(&rows, &three_models(), &recorded_dont, &drifted_map)
        .expect("routing succeeds");
    assert_eq!(proposals.len(), 1);
    let p = &proposals[0].to_json();
    assert_eq!(p["tool"], "dont", "repo keyed on recorded tool");
    assert_eq!(p["repo"], "../dont");
    // The drifted row tool survives as provenance only.
    assert_eq!(p["row_provenance"][0]["tool"], "wai");
}
