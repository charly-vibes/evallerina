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
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Isolation fixtures: ALL bd operations in these tests run inside a
// per-test temp dir created under std::env::temp_dir(). The trackers of
// evallerina and every family repo (wai/genesis/espectacular/dont) are
// NEVER touched. bd search-style dedup always pins --status open.
// ---------------------------------------------------------------------------

/// Serialize per-test fixture dirs even under `cargo test` parallelism.
fn fixture_counter() -> u64 {
    static COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
    let mut c = COUNTER.get_or_init(|| Mutex::new(0)).lock().unwrap();
    *c += 1;
    *c
}

/// An isolated temp sandbox. Deleted on drop.
struct Sandbox {
    path: PathBuf,
}

impl Sandbox {
    /// Fresh unique dir under std::env::temp_dir().
    fn fresh(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "evallerina-routing-{}-{}-{}",
            tag,
            std::process::id(),
            fixture_counter()
                + std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
        ));
        std::fs::create_dir_all(&path).expect("create sandbox dir");
        Sandbox { path }
    }

    /// A sandbox with beads initialized (`bd init` inside the sandbox).
    fn with_beads(tag: &str) -> Self {
        let sb = Sandbox::fresh(tag);
        routing::run_bd(&sb.path, &["init"]).expect("bd init in sandbox");
        sb
    }

    /// Create an issue via bd in this sandbox; returns the issue id.
    fn bd_create(&self, title: &str, body: &str) -> String {
        let out = routing::run_bd(&self.path, &["create", title, "-d", body, "--json"])
            .expect("bd create in sandbox");
        assert!(
            out.status.success(),
            "bd create failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("bd create --json");
        v["id"].as_str().expect("issue id").to_owned()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A proposal row ready for filing into `repo` (dry_run, mapped tool).
fn failable_row(token: &str, repo: &std::path::Path) -> routing::ProposalRow {
    routing::ProposalRow {
        tool: "wai".to_owned(),
        repo: Some(repo.to_string_lossy().into_owned()),
        scenario: "wai-hint-adherence-typo".to_owned(),
        fault_class: "tool_fault".to_owned(),
        token: token.to_owned(),
        decision: "dry_run".to_owned(),
        reason: None,
        bd_issue_id: None,
        row_provenance: vec![],
    }
}

/// A recording bd runner: delegates to the real `bd` (process boundary:
/// assert on exit codes + parsed ids) while capturing every invocation
/// so tests can assert on calls (e.g. zero `bd create`).
struct BdSpy {
    calls: Vec<(PathBuf, Vec<String>)>,
}

impl BdSpy {
    fn new() -> Self {
        BdSpy { calls: Vec::new() }
    }

    fn runner(
        &mut self,
    ) -> impl FnMut(&std::path::Path, &[&str]) -> Result<std::process::Output, String> + '_ {
        |cwd, args| {
            self.calls.push((
                cwd.to_path_buf(),
                args.iter().map(|s| s.to_string()).collect(),
            ));
            routing::run_bd(cwd, args)
        }
    }

    fn create_calls(&self) -> Vec<&(PathBuf, Vec<String>)> {
        self.calls
            .iter()
            .filter(|(_, a)| a.first().map(String::as_str) == Some("create"))
            .collect()
    }
}

impl Sandbox {
    /// All open issues in this sandbox, parsed from `bd list --status open --json`.
    fn list_open_issues(&self) -> Vec<serde_json::Value> {
        let out = routing::run_bd(&self.path, &["list", "--status", "open", "--json"])
            .expect("bd list in sandbox");
        assert!(
            out.status.success(),
            "bd list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("bd list --json array")
    }
}

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

// ── doc-coverage fault intake (evallerina-e4x) ─────────────────────────────

/// A bridged doc-audit report row (src/doc_audit.rs report_row shape
/// plus the routing intake attribution): one failing check per audit
/// row, carrying the check-code; `model` attributes the static audit
/// finding to one configured battery model.
fn doc_audit_row(tool: &str, model: &str, code: &str) -> serde_json::Value {
    json!({
        "kind": "doc_audit",
        "report_version": 1,
        "scenario": "doc-audit",
        "tool": tool,
        "model": model,
        "status": "failed",
        "checks": [{"name": "doc:sample-feature", "passed": false, "code": code}]
    })
}

#[test]
fn undocumented_feature_audit_row_crosses_threshold_to_dry_run_proposal() {
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| doc_audit_row("wai", m, "ERR_FEATURE_UNDOCUMENTED"))
        .collect();
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
        .expect("doc-audit rows route like any other fault row");
    assert_eq!(proposals.len(), 1, "one fault key above threshold");
    let p = &proposals[0].to_json();
    assert_eq!(p["scenario"], "doc-audit");
    assert_eq!(p["fault_class"], "ERR_FEATURE_UNDOCUMENTED");
    assert_eq!(p["token"], "evallerina:doc-audit:ERR_FEATURE_UNDOCUMENTED");
    assert_eq!(p["tool"], "wai", "repo keyed on the audited tool");
    assert_eq!(p["repo"], "../wai");
    assert_eq!(p["decision"], "dry_run");
    assert_eq!(p["reason"], serde_json::Value::Null);
    assert_eq!(p["bd_issue_id"], serde_json::Value::Null);
}

#[test]
fn unfindable_doc_audit_row_crosses_threshold_to_dry_run_proposal() {
    // Same routing shape for the second doc-coverage code: a token
    // documented somewhere but unreachable from the breadcrumb
    // channels is doc-authoring debt in the tool's repo.
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| doc_audit_row("wai", m, "ERR_DOC_NOT_FINDABLE"))
        .collect();
    let proposals = routing::propose(&rows, &three_models(), &recorded_wai, &repo_map())
        .expect("doc-audit rows route like any other fault row");
    assert_eq!(proposals.len(), 1);
    let p = &proposals[0].to_json();
    assert_eq!(p["fault_class"], "ERR_DOC_NOT_FINDABLE");
    assert_eq!(p["token"], "evallerina:doc-audit:ERR_DOC_NOT_FINDABLE");
    assert_eq!(p["tool"], "wai");
    assert_eq!(p["repo"], "../wai");
    assert_eq!(p["decision"], "dry_run");
}

#[test]
fn upstream_tool_audit_rows_are_skipped_never_filing_candidates() {
    // routing.toml marks third-party upstreams `upstream = true`: mapped
    // for parity completeness, but we never raise tickets in repos we
    // don't control. The proposal row is recorded (skipped), not dropped.
    let rows: Vec<serde_json::Value> = ["m/a", "m/b", "m/c"]
        .iter()
        .map(|m| doc_audit_row("bd", m, "ERR_FEATURE_UNDOCUMENTED"))
        .collect();
    let proposals =
        routing::propose(&rows, &three_models(), &recorded_wai, &repo_map_with_upstream())
            .expect("upstream tools are recorded, not dropped");
    assert_eq!(proposals.len(), 1);
    let p = &proposals[0].to_json();
    assert_eq!(p["decision"], "skipped");
    assert_eq!(p["reason"], "unmapped_upstream");

    // And filing NEVER touches an upstream row, even under --file:
    // zero bd calls, decision stays skipped.
    let mut row = proposals.into_iter().next().unwrap();
    let mut spy = BdSpy::new();
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy.runner())
        .expect("filing must not fail on an upstream row");
    assert_eq!(row.decision, "skipped", "upstream rows are never filing candidates");
    assert_eq!(row.reason.as_deref(), Some("unmapped_upstream"));
    assert!(spy.create_calls().is_empty(), "upstream tools are NEVER filed");
}

fn three_models() -> Vec<String> {
    vec!["m/a".into(), "m/b".into(), "m/c".into()]
}

fn recorded_wai(scenario: &str) -> Option<String> {
    (scenario == "wai-hint-adherence-typo").then(|| "wai".to_owned())
}

fn repo_map() -> std::collections::BTreeMap<String, evallerina::routing_parity::RoutingEntry> {
    [
        (
            "wai".to_owned(),
            evallerina::routing_parity::RoutingEntry {
                repo: "../wai".to_owned(),
                upstream: false,
            },
        ),
    ]
    .into()
}

fn repo_map_with_upstream() -> std::collections::BTreeMap<String, evallerina::routing_parity::RoutingEntry> {
    [
        (
            "wai".to_owned(),
            evallerina::routing_parity::RoutingEntry {
                repo: "../wai".to_owned(),
                upstream: false,
            },
        ),
        (
            "bd".to_owned(),
            evallerina::routing_parity::RoutingEntry {
                repo: "gastownhall/beads".to_owned(),
                upstream: true,
            },
        ),
    ]
    .into()
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
    drifted_map.insert(
        "dont".to_owned(),
        evallerina::routing_parity::RoutingEntry {
            repo: "../dont".to_owned(),
            upstream: false,
        },
    );
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
    let map = evallerina::routing_parity::parse_routing_entries(toml_src).expect("parses");
    assert_eq!(map.get("wai").map(|e| e.repo.as_str()), Some("../wai"));
    assert_eq!(map.get("genesis").map(|e| e.repo.as_str()), Some("../genesis"));
}

#[test]
fn routing_toml_missing_repo_is_loud() {
    let err = evallerina::routing_parity::parse_routing_entries("[wai]\ndescription = \"no repo\"\n")
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
    map.insert(
        "wai".to_owned(),
        evallerina::routing_parity::RoutingEntry {
            repo: "../wai".to_owned(),
            upstream: false,
        },
    );
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
fn open_issue_with_token_dedups_to_existing_without_creating() {
    let repo = Sandbox::with_beads("dedup-open");
    let token = "evallerina:wai-hint-adherence-typo:tool_fault";
    // Seed: an OPEN issue whose body carries the fingerprint token.
    let existing_id = repo.bd_create(
        "wai-hint-adherence-typo: tool_fault",
        &format!("fingerprint {token}"),
    );
    let _ = existing_id;

    let mut row = failable_row(token, &repo.path);
    let mut spy = BdSpy::new();
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy.runner())
        .expect("filing must not fail on a dedup hit");

    assert_eq!(row.decision, "existing", "open issue with token → existing");
    assert_eq!(row.reason, None);
    assert!(
        spy.create_calls().is_empty(),
        "dedup must invoke bd create zero times"
    );
    // The dedup search pins the status filter (never searches closed).
    assert!(
        spy.calls
            .iter()
            .any(|(_, a)| a.contains(&"--status".to_owned()) && a.contains(&"open".to_owned())),
        "dedup search must pass --status open"
    );
    assert_eq!(row.bd_issue_id, None, "bd_issue_id is set only for filed");
}

#[test]
fn closed_issue_does_not_block_refiling_a_regressed_fault() {
    // bd search includes closed issues by default — the status filter
    // must be pinned so a fault that regresses after its ticket was
    // closed is re-filed rather than suppressed.
    let repo = Sandbox::with_beads("dedup-closed");
    let token = "evallerina:wai-hint-adherence-typo:tool_fault";
    let closed_id = repo.bd_create(
        "wai-hint-adherence-typo: tool_fault",
        &format!("fingerprint {token}"),
    );
    let out = routing::run_bd(&repo.path, &["close", &closed_id]).expect("bd close");
    assert!(
        out.status.success(),
        "bd close failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut row = failable_row(token, &repo.path);
    let mut spy = BdSpy::new();
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy.runner())
        .expect("filing must not fail when only a closed issue holds the token");

    assert_eq!(row.decision, "filed", "closed must not block re-filing");
    assert_eq!(row.reason, None);
    let new_id = row
        .bd_issue_id
        .as_deref()
        .expect("filed captures the new issue id");
    assert_ne!(new_id, closed_id, "a NEW issue was filed");
    assert_eq!(spy.create_calls().len(), 1, "exactly one bd create");
}

#[test]
fn filing_invokes_bd_create_in_target_repo_cwd_and_captures_the_id() {
    let repo = Sandbox::with_beads("create-cwd");
    let token = "evallerina:wai-hint-adherence-typo:tool_fault";
    let mut row = failable_row(token, &repo.path);
    let mut spy = BdSpy::new();
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy.runner())
        .expect("filing in a beads-initialized repo must succeed");

    assert_eq!(row.decision, "filed");
    assert_eq!(row.reason, None);
    let id = row
        .bd_issue_id
        .as_deref()
        .expect("exit code + issue id captured");
    // bd derives the issue prefix from the repo's directory name — the
    // id must carry the TARGET repo's prefix, not evallerina's.
    let repo_prefix = repo.path.file_name().unwrap().to_string_lossy();
    assert!(
        id.starts_with(&*repo_prefix),
        "id belongs to the TARGET repo's tracker: {id}"
    );

    // Process-boundary: the create ran INSIDE the target repo cwd.
    let (cwd, args) = spy.create_calls()[0];
    assert_eq!(
        cwd, &repo.path,
        "bd create runs with the target repo as cwd"
    );
    assert!(
        args.contains(&"--json".to_owned()),
        "create is machine-parsed"
    );
    // The fingerprint token goes in the ticket BODY.
    let body_idx = args
        .iter()
        .position(|a| a == "-d")
        .expect("description flag present");
    assert!(
        args[body_idx + 1].contains(token),
        "token in the ticket body"
    );

    // Cross-check through bd itself: the issue exists, open, with the token.
    let issues = repo.list_open_issues();
    assert!(
        issues.iter().any(|i| i["id"].as_str() == Some(id)),
        "filed issue exists in target repo"
    );
}

#[test]
fn filing_refreshes_tracked_export_in_target_repo() {
    // Tracked-export invariant: after a successful create, the target
    // repo's .beads/issues.jsonl is the fresh source of truth.
    let repo = Sandbox::with_beads("create-export");
    let token = "evallerina:wai-hint-adherence-typo:tool_fault";
    let mut row = failable_row(token, &repo.path);
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy_noop())
        .expect("filing succeeds");
    let id = row.bd_issue_id.expect("filed");

    let jsonl_path = repo.path.join(".beads/issues.jsonl");
    let jsonl =
        std::fs::read_to_string(&jsonl_path).expect("tracked export refreshed after create");
    assert!(
        jsonl.contains(&id),
        "export contains the newly filed issue {id}"
    );
}

fn spy_noop() -> impl FnMut(&std::path::Path, &[&str]) -> Result<std::process::Output, String> {
    routing::run_bd
}

#[test]
fn bd_failure_in_target_repo_is_skipped_never_fatal() {
    // Real non-beads dir (no bd init): bd create fails. The decision is
    // skipped/bd_failure and filing completes — a broken tracker must
    // not crash the rotation or mask what did not happen.
    let not_a_beads_repo = Sandbox::fresh("bd-failure");
    let token = "evallerina:wai-hint-adherence-typo:tool_fault";
    let mut row = failable_row(token, &not_a_beads_repo.path);

    // Return value is Ok: no error propagated to the report.
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy_noop())
        .expect("bd failure must never propagate as an error");

    assert_eq!(row.decision, "skipped");
    assert_eq!(row.reason.as_deref(), Some("bd_failure"));
    assert_eq!(row.bd_issue_id, None, "no filing to claim");

    // Other rows still processed after one failure.
    let repo = Sandbox::with_beads("bd-failure-good");
    let failing = failable_row("evallerina:fault-a:tool_fault", &not_a_beads_repo.path);
    let healthy = failable_row("evallerina:fault-b:tool_fault", &repo.path);
    let mut rows = [failing, healthy];
    routing::file_proposals(&mut rows, true, &mut spy_noop()).expect("mixed repos still file");
    let [failing, healthy] = &rows;
    assert_eq!(failing.decision, "skipped");
    assert_eq!(failing.reason.as_deref(), Some("bd_failure"));
    assert_eq!(
        healthy.decision, "filed",
        "a healthy repo still files after a sibling failure"
    );
    assert!(healthy.bd_issue_id.is_some());
}

#[test]
fn bd_search_spawns_may_fail_too_but_dedup_falls_back() {
    // bd cannot even list here (no beads workspace): dedup finds
    // nothing, create then fails => skipped/bd_failure, not an error.
    let not_a_beads_repo = Sandbox::fresh("search-failure");
    let mut row = failable_row("evallerina:sc:tool_fault", &not_a_beads_repo.path);
    routing::file_proposals(std::slice::from_mut(&mut row), true, &mut spy_noop())
        .expect("search failure must not propagate");
    assert_eq!(row.decision, "skipped");
    assert_eq!(row.reason.as_deref(), Some("bd_failure"));
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
