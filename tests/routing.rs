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
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
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
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
        .expect("routing succeeds");
    assert!(
        proposals.is_empty(),
        "below threshold → recorded, no action"
    );
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
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
        .expect("routing succeeds");
    assert!(
        proposals.is_empty(),
        "agent faults must not produce proposals"
    );
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

#[test]
fn routing_toml_parses_tools_to_repos() {
    let toml_src = "[wai]\nrepo = \"../wai\"\n\n[genesis]\nrepo = \"../genesis\"\n";
    let map = routing::parse_routing_toml(toml_src).expect("parses");
    assert_eq!(map.get("wai").map(String::as_str), Some("../wai"));
    assert_eq!(map.get("genesis").map(String::as_str), Some("../genesis"));
}

#[test]
fn routing_toml_missing_repo_is_loud() {
    let err = routing::parse_routing_toml("[wai]\ndescription = \"no repo\"\n")
        .expect_err("missing repo is a loud error");
    assert!(err.contains("repo"));
}

#[test]
fn relative_repo_resolves_against_evallerina_root() {
    let root = std::path::Path::new("/worktrees/evallerina");
    assert_eq!(
        routing::resolve_repo("../wai", root),
        "/worktrees/evallerina/../wai"
    );
}

#[test]
fn absolute_repo_passes_through_for_worktrees() {
    let root = std::path::Path::new("/worktrees/evallerina");
    assert_eq!(
        routing::resolve_repo("/para/worktrees/wai", root),
        "/para/worktrees/wai"
    );
}

#[test]
fn every_proposal_row_carries_the_full_schema() {
    // Mixed bag: one mapped tool fault (dry_run), one unmapped
    // (skipped/unmapped_tool) — every row must carry ALL schema fields
    // with null where absent, and the JSONL artifact must be one
    // machine-checkable JSON object per line.
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .flat_map(|m| {
            vec![
                trial_row("wai-hint-adherence-typo", "wai", m),
                trial_row("wai-doc-drift-stale-agents-md", "wai", m),
            ]
        })
        .collect();
    let mut map = std::collections::BTreeMap::new();
    map.insert("wai".to_owned(), "../wai".to_owned());
    let two_scenarios = |s: &str| {
        (s == "wai-hint-adherence-typo" || s == "wai-doc-drift-stale-agents-md")
            .then(|| "wai".to_owned())
    };
    let proposals =
        routing::propose(&rows, &three_models(), &two_scenarios, &map).expect("routing succeeds");
    assert_eq!(proposals.len(), 2);
    for p in &proposals {
        let j = p.to_json();
        for field in [
            "tool",
            "repo",
            "scenario",
            "fault_class",
            "token",
            "decision",
            "reason",
            "bd_issue_id",
            "row_provenance",
        ] {
            assert!(j.get(field).is_some(), "schema field {field} missing");
        }
        assert_eq!(
            j["bd_issue_id"],
            serde_json::Value::Null,
            "dry-run files nothing"
        );
        assert_eq!(
            j["reason"],
            serde_json::Value::Null,
            "dry_run has no reason"
        );
        // row_provenance dedupes by model: 3 entries per proposal.
        assert_eq!(j["row_provenance"].as_array().map(Vec::len), Some(3));
        for prov in j["row_provenance"].as_array().unwrap() {
            for f in ["scenario", "model", "tool"] {
                assert!(prov.get(f).is_some(), "provenance field {f} missing");
            }
        }
    }
}

#[test]
fn unknown_scenario_is_a_loud_error() {
    let rows = vec![trial_row("mystery-scenario", "wai", "m/a")];
    let err = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
        .expect_err("unknown scenario must be loud, never a silent bucket");
    assert!(err.contains("mystery-scenario"));
}

#[test]
fn jsonl_writer_emits_one_object_per_line() {
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| trial_row("wai-hint-adherence-typo", "wai", m))
        .collect();
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
        .expect("routing succeeds");
    let jsonl = routing::to_jsonl(&proposals);
    let lines: Vec<&str> = jsonl.lines().collect();
    assert_eq!(lines.len(), proposals.len(), "one object per line");
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line).expect("each line is a JSON object");
        assert!(v.get("token").is_some());
    }
}
