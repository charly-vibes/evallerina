//! Purpose: Contract tests for the tier-1/2 report aggregation
//! dashboard (evallerina-48g).
//! Responsibilities: pin the report_version 1 aggregation contract —
//! per-output-channel × check-code × model-id fault matrix, evals-
//! guidelines fault routing (tool faults → tool tickets; agent faults
//! → AIX-channel work only at the cross-model threshold), and the
//! anti-goal that absent and rate_limited cells are surfaced, never
//! dropped or silently substituted. No single aggregate score exists.
//! Rationale: the dashboard is the consumer-side aggregation the
//! evals-guidelines spec requires; these tests are the executable
//! form of the spec's aggregation and fault-routing scenarios.

use serde_json::json;

use evallerina::registry::REGISTRY;
use evallerina::report::{DASHBOARD_VERSION, aggregate_rows};
use evallerina::scenario::live_scenarios;

/// Test channel map: hermetic — scenario name → the output channel the
/// scenario's checks probe (suggestions | envelope | managed-block |
/// artifact).
fn test_channel_of(name: &str) -> Option<&'static str> {
    match name {
        "wai-hint-adherence-smoke" => Some("envelope"),
        "wai-hint-adherence-typo" => Some("suggestions"),
        "wai-hint-adherence-corrupt-config" => Some("envelope"),
        "dont-hint-adherence-lifecycle" => Some("envelope"),
        "wai-doc-drift-stale-agents-md" | "dont-doc-drift-stale-lifecycle" => Some("artifact"),
        _ => None,
    }
}

/// A trial row (report_version 1 shape, as written by the tier-2
/// runner): `scenario` probes `channel`, `checks` carry the faults.
fn trial(
    scenario: &str,
    model: &str,
    status: &str,
    checks: serde_json::Value,
) -> serde_json::Value {
    json!({
        "kind": "trial",
        "report_version": 1,
        "scenario": scenario,
        "tool": "wai",
        "status": status,
        "checks": checks,
        "model": model,
        "repetition": 0
    })
}

/// One agent-fault check row.
fn agent_fault(code: &str) -> serde_json::Value {
    json!({"name": "check-a", "passed": false, "code": code, "reason": "hint ignored"})
}

/// One tool-fault check row (no code, per the contract).
fn tool_fault() -> serde_json::Value {
    json!({"name": "check-b", "passed": false, "reason": "envelope missing"})
}

fn models(n: usize) -> Vec<String> {
    REGISTRY.iter().take(n).map(|e| e.id.to_owned()).collect()
}

// ── matrix ──────────────────────────────────────────────────────────────────

#[test]
fn matrix_groups_agent_faults_by_channel_code_model() {
    let rows = vec![
        trial(
            "wai-hint-adherence-typo",
            "m1",
            "failed",
            json!([agent_fault("ERR_TOOL_DISCOVERY_FAILURE")]),
        ),
        trial(
            "wai-hint-adherence-typo",
            "m2",
            "failed",
            json!([agent_fault("ERR_TOOL_DISCOVERY_FAILURE")]),
        ),
    ];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    let cells = &dashboard["matrix"];
    assert_eq!(cells.as_array().map(Vec::len), Some(2));
    for cell in cells.as_array().unwrap() {
        assert_eq!(cell["channel"], "suggestions");
        assert_eq!(cell["code"], "ERR_TOOL_DISCOVERY_FAILURE");
        assert_eq!(cell["occurrences"], 1);
    }
    let mut got: Vec<&str> = cells
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["model"].as_str().unwrap())
        .collect();
    got.sort_unstable();
    assert_eq!(got, ["m1", "m2"]);
}

/// The same code in the same channel on different channels' scenarios
/// does NOT merge: the channel is the routing key, not the scenario.
#[test]
fn same_code_across_different_channels_stays_split() {
    let rows = vec![
        trial(
            "wai-hint-adherence-typo",
            "m1",
            "failed",
            json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
        ),
        trial(
            "wai-hint-adherence-smoke",
            "m1",
            "failed",
            json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
        ),
    ];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    let cells = dashboard["matrix"].as_array().unwrap();
    assert_eq!(cells.len(), 2);
    let mut channels: Vec<&str> = cells
        .iter()
        .map(|c| c["channel"].as_str().unwrap())
        .collect();
    channels.sort_unstable();
    assert_eq!(channels, ["envelope", "suggestions"]);
}

/// Repeated occurrences in the same cell accumulate.
#[test]
fn matrix_cell_accumulates_occurrences() {
    let rows = vec![
        trial(
            "wai-hint-adherence-typo",
            "m1",
            "failed",
            json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
        ),
        trial(
            "wai-hint-adherence-typo",
            "m1",
            "failed",
            json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
        ),
    ];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    assert_eq!(dashboard["matrix"][0]["occurrences"], 2);
}

/// A trial-level `invalid_output` carries ERR_ACTION_FORMAT_VIOLATION
/// and lands in the matrix as an agent-fault occurrence.
#[test]
fn invalid_output_lands_in_matrix_with_action_format_violation() {
    let rows = vec![trial(
        "wai-hint-adherence-typo",
        "m1",
        "invalid_output",
        json!([]),
    )];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    let cells = dashboard["matrix"].as_array().unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0]["code"], "ERR_ACTION_FORMAT_VIOLATION");
    assert_eq!(cells[0]["channel"], "suggestions");
    assert_eq!(cells[0]["model"], "m1");
    assert_eq!(cells[0]["occurrences"], 1);
}

// ── fault routing (evals-guidelines) ────────────────────────────────────────

#[test]
fn same_code_across_threshold_models_routes_to_aix() {
    let rows = ["m1", "m2", "m3"]
        .iter()
        .map(|m| {
            trial(
                "wai-hint-adherence-typo",
                m,
                "failed",
                json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
            )
        })
        .collect::<Vec<_>>();
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    let routings = dashboard["routing"].as_array().unwrap();
    let aix: Vec<_> = routings
        .iter()
        .filter(|r| r["kind"] == "aix-channel")
        .collect();
    assert_eq!(aix.len(), 1);
    assert_eq!(aix[0]["channel"], "suggestions");
    assert_eq!(aix[0]["code"], "ERR_DOC_DRIFT_BLINDNESS");
    assert_eq!(aix[0]["distinct_models"], 3);
}

#[test]
fn sub_threshold_code_recorded_without_action() {
    let rows = ["m1", "m2"]
        .iter()
        .map(|m| {
            trial(
                "wai-hint-adherence-typo",
                m,
                "failed",
                json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
            )
        })
        .collect::<Vec<_>>();
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    let routings = dashboard["routing"].as_array().unwrap();
    assert!(routings.iter().all(|r| r["kind"] != "aix-channel"));
    assert!(routings.iter().any(|r| r["kind"] == "recorded-no-action"
        && r["code"] == "ERR_DOC_DRIFT_BLINDNESS"
        && r["distinct_models"] == 2));
}

/// Small registries can still route: the threshold collapses to the
/// number of configured ids when fewer than three.
#[test]
fn small_registry_routes_when_all_configured_ids_hit() {
    let rows = ["m1", "m2"]
        .iter()
        .map(|m| {
            trial(
                "wai-hint-adherence-typo",
                m,
                "failed",
                json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
            )
        })
        .collect::<Vec<_>>();
    let dashboard = aggregate_rows(&rows, &models(2), &test_channel_of).expect("aggregates");
    let routings = dashboard["routing"].as_array().unwrap();
    assert!(
        routings
            .iter()
            .any(|r| r["kind"] == "aix-channel" && r["distinct_models"] == 2)
    );
    // And the dashboard states the collapsed threshold.
    assert_eq!(dashboard["routing_threshold"], 2);
}

#[test]
fn tool_fault_routes_to_tool_ticket_never_aix() {
    let rows = ["m1", "m2", "m3"]
        .iter()
        .map(|m| {
            trial(
                "wai-hint-adherence-typo",
                m,
                "failed",
                json!([tool_fault()]),
            )
        })
        .collect::<Vec<_>>();
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    let routings = dashboard["routing"].as_array().unwrap();
    let tickets: Vec<_> = routings
        .iter()
        .filter(|r| r["kind"] == "tool-ticket")
        .collect();
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0]["tool"], "wai");
    assert_eq!(tickets[0]["check"], "check-b");
    assert_eq!(tickets[0]["occurrences"], 3);
    // A tool fault is never AIX work, no matter how many models hit it.
    assert!(routings.iter().all(|r| r["kind"] != "aix-channel"));
    // Tool faults carry no code, per the report contract.
    assert!(tickets[0].get("code").is_none());
}

// ── coverage: absent / rate_limited / non-tier-2 (anti-goal) ────────────────

#[test]
fn absent_cells_surfaced_never_failed() {
    let rows = vec![
        trial(
            "wai-hint-adherence-typo",
            "m1",
            "failed",
            json!([tool_fault()]),
        ),
        json!({
            "kind": "absent",
            "scenario": "wai-hint-adherence-typo",
            "model": "m2",
            "repetition": 1
        }),
    ];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    assert_eq!(dashboard["coverage"]["trials"], 1);
    assert_eq!(dashboard["coverage"]["failed"], 1);
    assert_eq!(dashboard["coverage"]["absent"], 1);
    // The absent record is not a failure anywhere.
    assert_eq!(dashboard["coverage"]["passed"], 0);
}

#[test]
fn rate_limited_rows_surfaced_without_fault() {
    let rows = vec![trial(
        "wai-hint-adherence-typo",
        "m1",
        "rate_limited",
        json!([]),
    )];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    assert_eq!(dashboard["coverage"]["rate_limited"], 1);
    assert!(dashboard["matrix"].as_array().unwrap().is_empty());
}

/// Rows without model attribution group as non-tier-2 (evals-guidelines
/// suite-wide aggregation).
#[test]
fn rows_without_model_attribution_group_as_non_tier2() {
    let rows = vec![json!({
        "report_version": 1,
        "scenario": "wai-hint-adherence-smoke",
        "tool": "wai",
        "status": "passed",
        "checks": []
    })];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    assert_eq!(dashboard["coverage"]["non_tier2"], 1);
    assert_eq!(dashboard["coverage"]["trials"], 0);
    assert!(dashboard["matrix"].as_array().unwrap().is_empty());
}

// ── dashboard shape ─────────────────────────────────────────────────────────

/// The anti-goal is a single aggregate score: a dashboard that collapses
/// channel × code × model detail into one number hides exactly the
/// routing signal. The dashboard carries no score field.
#[test]
fn dashboard_has_no_single_aggregate_score() {
    let rows = vec![trial(
        "wai-hint-adherence-typo",
        "m1",
        "failed",
        json!([tool_fault()]),
    )];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    assert!(dashboard.get("score").is_none());
    assert!(dashboard.get("aggregate").is_none());
    assert_eq!(dashboard["dashboard_version"], DASHBOARD_VERSION);
    assert_eq!(dashboard["report_version"], 1);
    assert_eq!(dashboard["routing_threshold"], 3);
    assert_eq!(
        dashboard["configured_models"].as_array().map(Vec::len),
        Some(3)
    );
}

/// Passing checks are visible in coverage, not just faults.
#[test]
fn passing_checks_count_in_coverage() {
    let rows = vec![
        trial(
            "wai-hint-adherence-smoke",
            "m1",
            "passed",
            json!([{"name": "check-a", "passed": true}]),
        ),
        trial(
            "wai-hint-adherence-smoke",
            "m2",
            "passed",
            json!([{"name": "check-a", "passed": true}]),
        ),
    ];
    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
    assert_eq!(dashboard["coverage"]["passed"], 2);
    assert_eq!(dashboard["coverage"]["checks_passed"], 2);
    assert_eq!(dashboard["coverage"]["checks_failed"], 0);
}

// ── loud failures over silent grouping ──────────────────────────────────────

/// An unknown scenario has no channel mapping; silently bucketing it
/// would corrupt the routing key. Aggregate refuses loudly.
#[test]
fn unknown_scenario_channel_is_an_error() {
    let rows = vec![trial(
        "no-such-scenario",
        "m1",
        "failed",
        json!([tool_fault()]),
    )];
    assert!(aggregate_rows(&rows, &models(3), &test_channel_of).is_err());
}

/// An empty configured registry cannot define a routing threshold.
#[test]
fn empty_configured_registry_is_an_error() {
    let rows = vec![trial(
        "wai-hint-adherence-typo",
        "m1",
        "failed",
        json!([tool_fault()]),
    )];
    let empty: Vec<String> = Vec::new();
    assert!(aggregate_rows(&rows, &empty, &test_channel_of).is_err());
}

// ── registry wiring ─────────────────────────────────────────────────────────

/// Every live scenario must declare the output channel its checks
/// probe — the routing key has to exist before rotation day, not after
/// the first dashboard run.
#[test]
fn every_live_scenario_declares_a_channel() {
    for scenario in live_scenarios() {
        assert!(
            evallerina::report::scenario_channel(&scenario.name).is_some(),
            "live scenario '{}' has no output-channel declaration",
            scenario.name
        );
    }
}
