//! Purpose: Dry-run fault routing — proposals for tool-fault tickets in
//! the faulting tool's repository (openspec add-fault-ticket-routing).
//! Responsibilities: filter tier-2 report rows to tool faults (failing
//! checks without an `ERR_*` code), tally DISTINCT models per fault key
//! (scenario, fault-class, tool) using the same threshold semantics as
//! report.rs (min(3, |configured model registry|)), key the target
//! repository on the scenario definition's recorded tool (never the
//! report row's tool field, which is hardcoded for live trials), and
//! emit proposal rows with the fingerprint token
//! `evallerina:<scenario>:<fault-class>` and the full machine-checkable
//! schema. Phase 1 is dry-run only: every emitted decision is `dry_run`
//! (mapped tool) or `skipped`/`unmapped_tool` — no `bd` invocations
//! happen in the dry-run path. Phase 2 (evallerina-2bz) adds the filing
//! path behind explicit authorization: fingerprint-token dedup in the
//! target repo (`--status open` always pinned — closed issues must not
//! block re-filing a regressed fault), then `bd create` in the target
//! repo cwd with exit-code + id capture, then the tracked-export
//! invariant (`bd export -o .beads/issues.jsonl`). Every bd invocation
//! is routed through `run_bd` (cwd-scoped subprocess) and is injectable
//! so tests can spy on calls; `bd` failure is `skipped`/`bd_failure`,
//! never fatal — filing is an optimization over the report.
//! Rationale: the fault taxonomy routes tool faults to tickets in the
//! tool's repo, but the value that made evallerina useful (evallerina-2rr,
//! -00n, -54f) all required hand-filing from dashboard findings. Routing
//! makes that mechanical, with a dry-run default so cross-repo writes
//! stay behind an explicit local opt-in, and loud errors on unknown
//! scenarios (mirroring report.rs: never a silent bucket).

use std::collections::BTreeMap;

/// The cross-model routing threshold — identical semantics to
/// report.rs: a fault key observed in trial rows from at least this
/// many distinct models (3, or the full configured registry when
/// smaller) is routed.
const ROUTING_THRESHOLD: usize = 3;

/// One dry-run proposal row (the proposed-tickets.jsonl schema, design
/// §6): machine-checkable so the CI artifact is reviewable without
/// reading code.
#[derive(Debug, PartialEq)]
pub struct ProposalRow {
    /// The scenario definition's recorded tool — the repo key.
    pub tool: String,
    /// Mapped repository path (null until/unless the tool is mapped).
    pub repo: Option<String>,
    pub scenario: String,
    pub fault_class: String,
    /// Fingerprint token: `evallerina:<scenario>:<fault-class>`.
    pub token: String,
    /// `dry_run` (mapped, not filed) or `skipped` (unmapped tool).
    pub decision: String,
    /// Null unless skipped (`unmapped_tool`).
    pub reason: Option<String>,
    /// Null in dry-run; set only when a ticket is filed (Phase 2).
    pub bd_issue_id: Option<String>,
    /// Provenance of the contributing trial rows (deduped by model):
    /// report row scenario/model/tool values.
    pub row_provenance: Vec<ProvenanceRow>,
}

/// One contributing trial row's provenance.
#[derive(Debug, PartialEq)]
pub struct ProvenanceRow {
    pub scenario: String,
    pub model: String,
    /// The report row's tool value — provenance only, never the repo key.
    pub tool: String,
}

impl ProposalRow {
    /// Serialize to the proposed-tickets.jsonl JSON object.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "tool": self.tool,
            "repo": self.repo,
            "scenario": self.scenario,
            "fault_class": self.fault_class,
            "token": self.token,
            "decision": self.decision,
            "reason": self.reason,
            "bd_issue_id": self.bd_issue_id,
            "row_provenance": self.row_provenance.iter()
                .map(|p| serde_json::json!({
                    "scenario": p.scenario,
                    "model": p.model,
                    "tool": p.tool,
                }))
                .collect::<Vec<_>>(),
        })
    }
}

/// Propose dry-run tickets from tier-2 report rows.
///
/// `rows` are parsed report rows (same inputs as report.rs: tier-2
/// trial events, tier-1 rows without model attribution, absent records).
/// `configured_models` is the model registry the rotation was configured
/// with — it defines the routing threshold (min(3, registry size));
/// empty is a loud error, no routing threshold exists. `scenario_tool_of`
/// resolves a scenario to the tool its *definition* recorded (proposals
/// key the repo on this, never on the row's tool field); an unknown
/// scenario is a loud error. `repo_map` maps tool name → repo path (the
/// parsed routing.toml); a mapped-but-unregistered tool surfaces as
/// `skipped`/`unmapped_tool`, never a silent drop.
pub fn propose(
    rows: &[serde_json::Value],
    configured_models: &[String],
    scenario_tool_of: &dyn Fn(&str) -> Option<String>,
    repo_map: &BTreeMap<String, String>,
) -> Result<Vec<ProposalRow>, String> {
    if configured_models.is_empty() {
        return Err("configured model registry is empty — no routing threshold exists".into());
    }
    let routing_threshold = configured_models.len().min(ROUTING_THRESHOLD);

    // Fault key (scenario, fault-class, tool-from-scenario) →
    // distinct models + occurrences + provenance rows.
    let mut faults: BTreeMap<FaultKey, FaultTally> = BTreeMap::new();

    for row in rows {
        let kind = row.get("kind").and_then(serde_json::Value::as_str);
        if kind == Some("absent") {
            // Absent cells are the record that a cell did not run;
            // never failures, never dropped (anti-goal — mirrored
            // from report.rs).
            continue;
        }
        let scenario = row
            .get("scenario")
            .and_then(serde_json::Value::as_str)
            .ok_or("report row lacks a `scenario` field")?;
        let row_tool = row
            .get("tool")
            .and_then(serde_json::Value::as_str)
            .ok_or("report row lacks a `tool` field")?;
        let report_version = row
            .get("report_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or("report row lacks a `report_version` field")?;
        if report_version != 1 {
            return Err(format!(
                "report row carries report_version {report_version}; routing aggregates version 1 only"
            ));
        }
        let checks = row
            .get("checks")
            .and_then(serde_json::Value::as_array)
            .ok_or("report row lacks a `checks` array")?;

        let model = row.get("model").and_then(serde_json::Value::as_str);
        for check in checks {
            // Validated for loudness even though routing keys on the
            // scenario, not the check name (report.rs keys the matrix
            // on it; routing's ticket granularity is the scenario).
            let _name = check
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("check row lacks a `name` field")?;
            let passed = check
                .get("passed")
                .and_then(serde_json::Value::as_bool)
                .ok_or("check row lacks a `passed` field")?;
            if passed {
                continue;
            }
            match check.get("code").and_then(serde_json::Value::as_str) {
                // Doc-coverage faults (evallerina-fok, routing intake
                // per add-ecosystem-doc-evals): the two audit codes
                // route like any other fault row. The repo keys on the
                // row's `tool` field — doc-audit rows carry the audited
                // tool authoritatively (there is no scenario-definition
                // recorded tool for the static audit) — and the fault
                // class is the check code itself.
                Some(code) if code == crate::doc_audit::ERR_FEATURE_UNDOCUMENTED => {
                    let key = FaultKey {
                        scenario: scenario.to_owned(),
                        fault_class: code.to_owned(),
                        tool: row_tool.to_owned(),
                    };
                    let entry = faults.entry(key).or_default();
                    entry.occurrences += 1;
                    if let Some(model) = model {
                        entry.models.insert((model.to_owned(), row_tool.to_owned()));
                    }
                }
                // Agent faults are never routed (fault taxonomy: they
                // inform AIX-channel work in genesis — a human decision).
                Some(code) if code.starts_with("ERR_") => continue,
                // Tool fault: no code by contract (report.rs FaultKey::Tool).
                // The repo keys on the scenario definition's recorded
                // tool, not the row's tool field (hardcoded in live
                // rows; the hhs bug showed attribution drifts). Unknown
                // scenario → loud. Resolved lazily: doc-audit rows
                // never reach this arm, so the static audit needs no
                // scenario-registry entry.
                _ => {
                    let recorded_tool = scenario_tool_of(scenario).ok_or_else(|| {
                        format!("scenario '{scenario}' has no recorded tool — refusing to route")
                    })?;
                    let key = FaultKey {
                        scenario: scenario.to_owned(),
                        fault_class: "tool_fault".to_owned(),
                        tool: recorded_tool.to_owned(),
                    };
                    let entry = faults.entry(key).or_default();
                    entry.occurrences += 1;
                    if let Some(model) = model {
                        entry.models.insert((model.to_owned(), row_tool.to_owned()));
                    }
                }
            }
        }
    }

    Ok(faults
        .into_iter()
        .filter(|(_, tally)| tally.models.len() >= routing_threshold)
        .map(|(key, tally)| {
            let repo = repo_map.get(&key.tool).cloned();
            let (decision, reason) = match repo {
                Some(_) => ("dry_run".to_owned(), None),
                None => ("skipped".to_owned(), Some("unmapped_tool".to_owned())),
            };
            ProposalRow {
                token: format!("evallerina:{}:{}", key.scenario, key.fault_class),
                scenario: key.scenario.clone(),
                fault_class: key.fault_class.clone(),
                tool: key.tool,
                repo,
                decision,
                reason,
                bd_issue_id: None,
                row_provenance: tally
                    .models
                    .into_iter()
                    .map(|(model, tool)| ProvenanceRow {
                        scenario: key.scenario.clone(),
                        model,
                        tool,
                    })
                    .collect(),
            }
        })
        .collect())
}

/// Internal fault key: (scenario, fault-class, scenario-recorded tool).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FaultKey {
    scenario: String,
    fault_class: String,
    tool: String,
}

/// Accumulated occurrences for one fault key: distinct
/// (model, row-tool) pairs + total count. Distinct models drive the
/// threshold; the row tool rides along as provenance.
#[derive(Debug, Default)]
struct FaultTally {
    occurrences: usize,
    models: BTreeSet<(String, String)>,
}

use std::collections::BTreeSet;

/// Parse routing.toml content into the tool → repo map. Structure is
/// deliberately minimal: `[<tool>]` tables with a `repo` string.
pub fn parse_routing_toml(content: &str) -> Result<BTreeMap<String, String>, String> {
    let value: toml::Value =
        toml::from_str(content).map_err(|e| format!("routing.toml parse error: {e}"))?;
    let mut map = BTreeMap::new();
    let table = value
        .as_table()
        .ok_or("routing.toml must be a table of [tool] sections")?;
    for (tool, item) in table {
        let repo = item
            .get("repo")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("routing.toml [{tool}] lacks a `repo` string"))?;
        map.insert(tool.clone(), repo.to_owned());
    }
    Ok(map)
}

/// Resolve a mapped repo path: absolute paths pass through untouched
/// (target-repo worktrees); relative paths resolve against the
/// evallerina repo root (via `git rev-parse --show-toplevel`, so they
/// survive worktree checkouts of evallerina itself).
pub fn resolve_repo(repo: &str, evallerina_root: &std::path::Path) -> String {
    let path = std::path::Path::new(repo);
    if path.is_absolute() {
        return repo.to_owned();
    }
    evallerina_root.join(repo).to_string_lossy().into_owned()
}

/// Serialize proposals to the proposed-tickets.jsonl artifact: one JSON
/// object per line, machine-checkable, reviewable without reading code.
pub fn to_jsonl(proposals: &[ProposalRow]) -> String {
    let mut out = String::new();
    for p in proposals {
        out.push_str(&p.to_json().to_string());
        out.push('\n');
    }
    out
}

/// The result of one bd invocation: exit code + streams, or a spawn
/// error string. Process-boundary principle: callers assert on these,
/// never on bd internals.
pub type BdResult = Result<std::process::Output, String>;

/// Run `bd` with `args` in `cwd` (the target repo). Every bd operation
/// in routing happens through here so the cwd is always explicit and
/// tests can inject a spy.
pub fn run_bd(cwd: &std::path::Path, args: &[&str]) -> BdResult {
    std::process::Command::new("bd")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("cannot run bd: {e}"))
}

/// The bd runner abstraction passed to `file_proposals`.
pub type BdRunner<'a> = dyn FnMut(&std::path::Path, &[&str]) -> BdResult + 'a;

/// True when the bd output is a successful JSON array with ≥1 issue.
fn bd_issue_array(out: &std::process::Output) -> Option<Vec<serde_json::Value>> {
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
        .ok()
        .filter(|v| !v.is_empty())
}

/// True when any issue in the JSON array carries `token` in its title
/// or description (the fingerprint dedup match).
fn any_issue_holds_token(issues: &[serde_json::Value], token: &str) -> bool {
    issues.iter().any(|i| {
        let text = i["description"].as_str().unwrap_or("").to_owned()
            + "\n"
            + i["title"].as_str().unwrap_or("");
        text.contains(token)
    })
}

/// File authorized proposals into their mapped repos (Phase 2,
/// `--file`): fingerprint-token dedup, then `bd create` in the target
/// repo cwd. Mutates each row's decision/reason/bd_issue_id in place:
///
/// - open issue in the target repo holds the token → `existing` (zero
///   `bd create` calls; the dedup search always pins `--status open`,
///   so a closed ticket never suppresses a regressed fault),
/// - no open match → `bd create` there → `filed` with the new issue
///   id, followed by `bd export -o .beads/issues.jsonl` (tracked-export
///   invariant),
/// - `bd create` fails (not a beads repo, binary missing, locked) →
///   `skipped` with reason `bd_failure` — never an error return; filing
///   is an optimization over the report.
///
/// `file_authorized=false` is the dry-run default: no bd calls at all.
/// Rows already `skipped`/`unmapped_tool` or with no mapped repo are
/// untouched.
pub fn file_proposals(
    proposals: &mut [ProposalRow],
    file_authorized: bool,
    run_bd: &mut BdRunner,
) -> Result<(), String> {
    if !file_authorized {
        return Ok(());
    }
    for row in proposals.iter_mut() {
        if row.decision != "dry_run" {
            continue;
        }
        let Some(repo) = row.repo.clone() else {
            continue;
        };
        let cwd = std::path::PathBuf::from(&repo);

        // Dedup: `bd search <token> --status open --json` first (title
        // search); the token lives in the issue BODY, so fall back to
        // `bd list --status open --json` + grep — same status filter.
        let search = run_bd(&cwd, &["search", &row.token, "--status", "open", "--json"])
            .ok()
            .and_then(|out| bd_issue_array(&out));
        let found = match search {
            Some(issues) if any_issue_holds_token(&issues, &row.token) => true,
            _ => run_bd(&cwd, &["list", "--status", "open", "--json"])
                .ok()
                .and_then(|out| bd_issue_array(&out))
                .map(|issues| any_issue_holds_token(&issues, &row.token))
                .unwrap_or(false),
        };
        if found {
            row.decision = "existing".to_owned();
            row.reason = None;
            continue; // bd_issue_id stays null: set only for `filed`.
        }

        // File: `bd create` in the target repo cwd, exit code + id.
        let title = format!("[evallerina] {}: {}", row.fault_class, row.scenario);
        let body = format!(
            "{}\ntool: {}\nfiled by evallerina fault routing (openspec add-fault-ticket-routing)",
            row.token, row.tool
        );
        match run_bd(&cwd, &["create", &title, "-d", &body, "--json"]) {
            Ok(out) if out.status.success() => {
                let id = serde_json::from_slice::<serde_json::Value>(&out.stdout)
                    .ok()
                    .and_then(|v| v["id"].as_str().map(String::from));
                match id {
                    Some(id) => {
                        row.decision = "filed".to_owned();
                        row.reason = None;
                        row.bd_issue_id = Some(id);
                        // Tracked-export invariant: refresh the JSONL
                        // source of truth after a successful create.
                        // Export failure is not fatal to the report.
                        let _ = run_bd(&cwd, &["export", "-o", ".beads/issues.jsonl"]);
                    }
                    // Create succeeded but the id is unparseable — treat
                    // as a bd failure rather than claim a filing we
                    // cannot link (follow-up threshold: no per-version
                    // output-format parsers).
                    None => {
                        row.decision = "skipped".to_owned();
                        row.reason = Some("bd_failure".to_owned());
                    }
                }
            }
            Ok(_) | Err(_) => {
                row.decision = "skipped".to_owned();
                row.reason = Some("bd_failure".to_owned());
            }
        }
    }
    Ok(())
}

/// Discover the evallerina repo root (worktree-safe) for relative
/// routing.toml path resolution.
pub fn discover_root() -> Result<String, String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("cannot run git for repo root: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git rev-parse --show-toplevel failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
