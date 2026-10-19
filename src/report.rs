//! Purpose: Tier-1/2 report aggregation into the per-channel fault
//! dashboard (evallerina-48g).
//! Responsibilities: aggregate report_version 1 rows (tier-2 rotation
//! events and tier-1 rows without model attribution) into a
//! per-output-channel × check-code × model-id fault matrix, apply the
//! evals-guidelines fault routing (tool faults → tool tickets; agent
//! faults → AIX-channel work only at the cross-model threshold),
//! surface absent, rate_limited, and http_error cells as coverage
//! records — never failures, never dropped — and refuse loudly on unmappable scenarios
//! instead of silently bucketing them.
//! Rationale: evals-guidelines "Fault routing" and "suite-wide
//! aggregation" requirements. The dashboard deliberately carries no
//! single aggregate score: per-capability breakdown is a hard
//! requirement, and a collapse into one number hides exactly the
//! channel × code × model signal the routing rule needs.

use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// Validate the `report` subcommand's routing flags: `--file` (cross-repo
/// write authorization, Phase 2) requires `--route` — filing proposed
/// tickets only makes sense after the routing step ran. Any combination
/// without `--file` is fine.
pub fn validate_route_flags(route: bool, file: bool) -> Result<(), String> {
    if file && !route {
        return Err(
            "--file requires --route: filing proposed tickets only makes sense after the routing step ran".into(),
        );
    }
    Ok(())
}

/// The scenario definition's recorded tool — the repo key for routing
/// (routing.rs keys proposals on this, never the report row's `tool`
/// field, which is hardcoded for live trials). Scenario names
/// prefix-encode the eval target by convention (`wai-*`, `dont-*`);
/// an unknown prefix is None → a loud error downstream (never a
/// silent bucket, mirroring aggregate_rows).
pub fn scenario_tool_of(name: &str) -> Option<String> {
    if name.starts_with("wai-") {
        Some("wai".to_owned())
    } else if name.starts_with("dont-") {
        Some("dont".to_owned())
    } else {
        None
    }
}

/// Run the routing step against an aggregated dashboard: propose
/// dry-run tickets from the same rows the dashboard aggregated from,
/// optionally file them (Phase 2, `--file`), and attach the additive
/// `routed` summary — decision counts only, no aggregate score — to
/// the dashboard JSON.
///
/// Returns the proposals so the caller can serialize
/// proposed-tickets.jsonl. Routing never affects the exit code unless
/// it errors loudly (empty registry, unknown scenario, version
/// mismatch): skipped proposals (unmapped tool, bd failure) are the
/// record, not failures.
pub fn attach_routing(
    dashboard: &mut serde_json::Value,
    rows: &[serde_json::Value],
    configured_models: &[String],
    repo_map: &BTreeMap<String, String>,
    file_authorized: bool,
    run_bd: &mut crate::routing::BdRunner,
) -> Result<Vec<crate::routing::ProposalRow>, String> {
    let mut proposals =
        crate::routing::propose(rows, configured_models, &scenario_tool_of, repo_map)?;
    crate::routing::file_proposals(&mut proposals, file_authorized, run_bd)?;

    let (mut filed, mut existing, mut dry_run, mut skipped) = (0usize, 0, 0, 0);
    for p in &proposals {
        match p.decision.as_str() {
            "filed" => filed += 1,
            "existing" => existing += 1,
            "dry_run" => dry_run += 1,
            _ => skipped += 1,
        }
    }
    dashboard["routed"] = json!({
        "filed": filed,
        "existing": existing,
        "dry_run": dry_run,
        "skipped": skipped,
    });
    Ok(proposals)
}

/// The full CLI routing step for `report --route`: load routing.toml
/// (from the current directory — the repo root on rotation day),
/// attach the routed summary to the dashboard, and write
/// proposed-tickets.jsonl next to the report file. `file_authorized`
/// comes from the explicit `--file` flag; the runner is injectable so
/// tests can spy on bd calls.
pub fn run_routing_step(
    dashboard: &mut serde_json::Value,
    rows: &[serde_json::Value],
    configured_models: &[String],
    report_path: &std::path::Path,
    file_authorized: bool,
    run_bd: &mut crate::routing::BdRunner,
) -> Result<Vec<crate::routing::ProposalRow>, String> {
    let content = std::fs::read_to_string("routing.toml")
        .map_err(|e| format!("cannot read routing.toml: {e}"))?;
    let repo_map = crate::routing::parse_routing_toml(&content)?;
    let proposals = attach_routing(
        dashboard,
        rows,
        configured_models,
        &repo_map,
        file_authorized,
        run_bd,
    )?;
    let jsonl = crate::routing::to_jsonl(&proposals);
    let out_path = report_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("proposed-tickets.jsonl");
    std::fs::write(&out_path, jsonl)
        .map_err(|e| format!("cannot write {}: {e}", out_path.display()))?;
    Ok(proposals)
}

/// Version of the dashboard shape. Additive evolution only within a
/// version; breaking changes bump this.
pub const DASHBOARD_VERSION: u32 = 1;

/// The evals-guidelines cross-model routing threshold for agent faults
/// within one output channel.
const ROUTING_THRESHOLD: usize = 3;

/// The output channel a live scenario's checks probe (the fault-routing
/// key per evals-guidelines: suggestions, envelope, managed-block, or
/// artifact surface). Registered for every live scenario; the channel
/// must exist before rotation day, not after the first dashboard run.
pub fn scenario_channel(name: &str) -> Option<&'static str> {
    match name {
        "wai-hint-adherence-smoke"
        | "wai-hint-adherence-corrupt-config"
        | "dont-hint-adherence-lifecycle" => Some("envelope"),
        "wai-hint-adherence-typo" => Some("suggestions"),
        "wai-doc-drift-stale-agents-md" | "dont-doc-drift-stale-lifecycle" => Some("artifact"),
        _ => None,
    }
}

/// One accumulator key in the routing maps: agent faults key on
/// (channel, code), tool faults on (tool, check, channel) because they
/// carry no code by contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FaultKey {
    /// Agent fault: the `ERR_*` taxonomy code within one output channel.
    Agent { channel: String, code: String },
    /// Tool fault: no code; routed to the tool's repository.
    Tool {
        tool: String,
        check: String,
        channel: String,
    },
}

/// Accumulated occurrences for one fault key: total count plus the set
/// of distinct model ids observed (empty for non-tier-2 rows).
#[derive(Debug, Default)]
struct FaultTally {
    occurrences: usize,
    models: BTreeSet<String>,
    scenarios: BTreeSet<String>,
}

/// Aggregate report_version 1 rows into the dashboard JSON.
///
/// `rows` are parsed report rows as [`serde_json::Value`]: tier-2
/// rotation events (`kind: trial | absent`) or tier-1 rows without a
/// `model` field (grouped as non-tier-2 per the report contract).
/// `configured_models` is the model registry the rotation was
/// configured with — it defines the routing threshold (3, or the full
/// registry size when fewer than three are configured). `channel_of`
/// resolves a scenario to the output channel its checks probe; an
/// unknown scenario is a loud error, never a silent bucket.
///
/// The returned value is the serialized dashboard shape: no single
/// aggregate score, per-capability breakdown only.
pub fn aggregate_rows(
    rows: &[serde_json::Value],
    configured_models: &[String],
    channel_of: &dyn Fn(&str) -> Option<&'static str>,
) -> Result<serde_json::Value, String> {
    if configured_models.is_empty() {
        return Err("configured model registry is empty — no routing threshold exists".into());
    }
    let routing_threshold = configured_models.len().min(ROUTING_THRESHOLD);

    let mut matrix_cells: BTreeMap<(String, String, String), FaultTally> = BTreeMap::new();
    let mut routings: BTreeMap<FaultKey, FaultTally> = BTreeMap::new();
    let mut coverage = Coverage::default();
    let mut report_versions = BTreeSet::new();

    for row in rows {
        let kind = row.get("kind").and_then(serde_json::Value::as_str);
        if kind == Some("absent") {
            // Absent cells are the record that a cell did not run; they
            // are never failures and never dropped (anti-goal).
            coverage.absent += 1;
            continue;
        }
        let status = row
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or("report row lacks a `status` field")?;
        let scenario = row
            .get("scenario")
            .and_then(serde_json::Value::as_str)
            .ok_or("report row lacks a `scenario` field")?;
        let tool = row
            .get("tool")
            .and_then(serde_json::Value::as_str)
            .ok_or("report row lacks a `tool` field")?;
        let checks = row
            .get("checks")
            .and_then(serde_json::Value::as_array)
            .ok_or("report row lacks a `checks` array")?;
        let report_version = row
            .get("report_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or("report row lacks a `report_version` field")?;
        if report_version != 1 {
            return Err(format!(
                "report row carries report_version {report_version}; this dashboard aggregates version 1 only"
            ));
        }
        report_versions.insert(report_version);

        let channel = channel_of(scenario).ok_or_else(|| {
            format!("scenario '{scenario}' has no declared output channel — refusing to aggregate")
        })?;

        let model = row.get("model").and_then(serde_json::Value::as_str);
        match model {
            Some(_) => {
                coverage.trials += 1;
                bump_status(&mut coverage, status)?;
            }
            None => coverage.non_tier2 += 1,
        }

        for check in checks {
            let name = check
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("check row lacks a `name` field")?;
            let passed = check
                .get("passed")
                .and_then(serde_json::Value::as_bool)
                .ok_or("check row lacks a `passed` field")?;
            if passed {
                coverage.checks_passed += 1;
                continue;
            }
            coverage.checks_failed += 1;
            match check.get("code").and_then(serde_json::Value::as_str) {
                Some(code) => {
                    let key = FaultKey::Agent {
                        channel: channel.to_owned(),
                        code: code.to_owned(),
                    };
                    tally(
                        &mut routings,
                        &mut matrix_cells,
                        &key,
                        scenario,
                        model,
                        name,
                    );
                }
                None => {
                    let key = FaultKey::Tool {
                        tool: tool.to_owned(),
                        check: name.to_owned(),
                        channel: channel.to_owned(),
                    };
                    tally(
                        &mut routings,
                        &mut matrix_cells,
                        &key,
                        scenario,
                        model,
                        name,
                    );
                }
            }
        }

        // A trial-level `invalid_output` carries the agent fault
        // `ERR_ACTION_FORMAT_VIOLATION` (report contract) — it is a
        // fault occurrence in the scenario's channel, matrix-visible.
        // The mapping is fixed by the contract, so an explicit
        // `fault_code` field is honored when present and the contract
        // default applies otherwise (tier-1 rows may omit it).
        if status == "invalid_output" {
            let code = row
                .get("fault_code")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("ERR_ACTION_FORMAT_VIOLATION");
            let key = FaultKey::Agent {
                channel: channel.to_owned(),
                code: code.to_owned(),
            };
            tally(&mut routings, &mut matrix_cells, &key, scenario, model, "");
        }
    }

    let matrix: Vec<serde_json::Value> = matrix_cells
        .iter()
        .map(|((channel, code, model), tally)| {
            json!({
                "channel": channel,
                "code": code,
                "model": model,
                "occurrences": tally.occurrences,
                "scenarios": tally.scenarios.iter().collect::<Vec<_>>(),
            })
        })
        .collect();

    let routing: Vec<serde_json::Value> = routings
        .iter()
        .map(|(key, tally)| {
            let distinct_models = tally.models.len();
            match key {
                FaultKey::Agent { channel, code } => {
                    // Cross-model threshold: 3 distinct ids for the
                    // channel — or all configured ids when fewer than
                    // three are configured. Below the threshold the
                    // finding is recorded without action.
                    let kind = if distinct_models >= routing_threshold {
                        "aix-channel"
                    } else {
                        "recorded-no-action"
                    };
                    json!({
                        "kind": kind,
                        "channel": channel,
                        "code": code,
                        "distinct_models": distinct_models,
                        "occurrences": tally.occurrences,
                        "scenarios": tally.scenarios.iter().collect::<Vec<_>>(),
                    })
                }
                FaultKey::Tool {
                    tool,
                    check,
                    channel,
                } => {
                    // Tool faults are filed as tickets in the tool's
                    // repository, never routed as AIX-channel work,
                    // and carry no code by contract.
                    json!({
                        "kind": "tool-ticket",
                        "tool": tool,
                        "check": check,
                        "channel": channel,
                        "distinct_models": distinct_models,
                        "occurrences": tally.occurrences,
                        "scenarios": tally.scenarios.iter().collect::<Vec<_>>(),
                    })
                }
            }
        })
        .collect();

    Ok(json!({
        "dashboard_version": DASHBOARD_VERSION,
        "report_version": report_versions.iter().next().copied().unwrap_or(1),
        "configured_models": configured_models,
        "routing_threshold": routing_threshold,
        "matrix": matrix,
        "routing": routing,
        "coverage": coverage_json(&coverage),
    }))
}

/// Increment a check-row fault into the routing tally and, when the
/// row carries model attribution, into the tier-2 matrix cell.
fn tally(
    routings: &mut BTreeMap<FaultKey, FaultTally>,
    matrix_cells: &mut BTreeMap<(String, String, String), FaultTally>,
    key: &FaultKey,
    scenario: &str,
    model: Option<&str>,
    check: &str,
) {
    let entry = routings.entry(key.clone()).or_default();
    entry.occurrences += 1;
    entry.scenarios.insert(scenario.to_owned());
    if let Some(model) = model {
        entry.models.insert(model.to_owned());
        let code = match key {
            FaultKey::Agent { code, .. } => code.clone(),
            // Tool faults carry no code; the matrix dimension uses the
            // check name so tool rows stay visible per capability.
            FaultKey::Tool { .. } => check.to_owned(),
        };
        let cell = matrix_cells
            .entry((
                match key {
                    FaultKey::Agent { channel, .. } | FaultKey::Tool { channel, .. } => {
                        channel.clone()
                    }
                },
                code,
                model.to_owned(),
            ))
            .or_default();
        cell.occurrences += 1;
        cell.scenarios.insert(scenario.to_owned());
    }
}

/// Per-trial outcome status counts. Unknown statuses are a loud error:
/// a silently-substituted status would corrupt the coverage record.
fn bump_status(coverage: &mut Coverage, status: &str) -> Result<(), String> {
    match status {
        "passed" => coverage.passed += 1,
        "failed" => coverage.failed += 1,
        "rate_limited" => coverage.rate_limited += 1,
        "http_error" => coverage.http_error += 1,
        "invalid_output" => coverage.invalid_output += 1,
        other => return Err(format!("unknown trial status '{other}'")),
    }
    Ok(())
}

/// Coverage counters (the anti-goal record: absent and rate_limited
/// cells are surfaced, never dropped, never failures).
#[derive(Debug, Default)]
struct Coverage {
    trials: usize,
    passed: usize,
    failed: usize,
    rate_limited: usize,
    http_error: usize,
    invalid_output: usize,
    absent: usize,
    non_tier2: usize,
    checks_passed: usize,
    checks_failed: usize,
}

fn coverage_json(coverage: &Coverage) -> serde_json::Value {
    json!({
        "trials": coverage.trials,
        "passed": coverage.passed,
        "failed": coverage.failed,
        "rate_limited": coverage.rate_limited,
        "http_error": coverage.http_error,
        "invalid_output": coverage.invalid_output,
        "absent": coverage.absent,
        "non_tier2": coverage.non_tier2,
        "checks_passed": coverage.checks_passed,
        "checks_failed": coverage.checks_failed,
    })
}
