//! Purpose: integration tests for the tier-0 routing-map parity gate
//! (evallerina-0sy, openspec add-ecosystem-doc-evals D2).
//! Responsibilities: pin the ddl README parser (MANAGED-TOOLS block +
//! canonical-names table, name-mismatch rows included), the tool→repo
//! resolution chain (canonical table → binary-name fallback → explicit
//! routing.toml override), the gate's fail/warn matrix (unmapped
//! first-party tool, unresolved repo path, upstream entry missing,
//! absent ddl sibling → warning exit 0), and the upstream policy
//! (`bd`/`openspec` carry `upstream = true`, data only — filing
//! semantics belong to e4x). All fixture tests run against in-memory
//! strings and temp sandboxes; the real-workspace gate test pins the
//! checked-in routing.toml against the real ddl sibling.
//! Rationale: the gate is the mechanism that fails `just tier0` the
//! moment routing.toml drifts from ddl's MANAGED-TOOLS block — every
//! decision branch must be machine-checked, and parse failures must
//! never degrade to warnings (anti-goal: only a genuinely absent
//! sibling may warn).

use evallerina::routing_parity;
use std::collections::BTreeMap;

/// Fixture ddl README: managed block + canonical-names table, mirroring
/// the real `../dulce-de-leche/README.md` shape (tier annotations with
/// counts, prose lines inside the block, backticked table cells, and
/// the `ddl (this tool)` annotated binary cell).
const FIXTURE_README: &str = r#"# dulce-de-leche

<!-- MANAGED-TOOLS:START -->
Managed tools (5):
- Core (2): wai, testaruda.
- Recommended (1): turu.
- Extension (2): bd, openspec.
Run `ddl version` for the full picture. Additions and removals must update
this block (enforced by `tests/tool_registry_drift.rs`).
<!-- MANAGED-TOOLS:END -->

Canonical names and aliases — binary vs crate vs repo (agents: install by
either spelling, but the repo is always the third column):

| binary | crate | repository |
|---|---|---|
| `wai` | `wai-cli` | `charly-vibes/wai` |
| `turu` | `whisper-vibes` | `charly-vibes/whisper` |
| `openspec` | `@fission-ai/openspec` (npm) | `fission-ai/openspec` |
| `ddl` (this tool) | `dulce-de-leche` | `charly-vibes/dulce-de-leche` |

`charly-vibes/ddl` does not exist — the repository is
`charly-vibes/dulce-de-leche`. This table lists only the tools whose binary
and crate names mismatch.
"#;

// ---------------------------------------------------------------------------
// Step 1: the parser — MANAGED-TOOLS block + canonical-names table.
// ---------------------------------------------------------------------------

#[test]
fn managed_block_yields_the_tool_list_in_order() {
    let tools = routing_parity::parse_managed_tools(FIXTURE_README).expect("fixture parses");
    assert_eq!(
        tools,
        vec![
            "wai".to_owned(),
            "testaruda".to_owned(),
            "turu".to_owned(),
            "bd".to_owned(),
            "openspec".to_owned(),
        ]
    );
}

#[test]
fn canonical_table_resolves_name_mismatch_rows() {
    let table = routing_parity::parse_canonical_repos(FIXTURE_README).expect("fixture parses");
    // Name-mismatch rows resolve through the table's repository column.
    assert_eq!(
        table.get("turu").map(String::as_str),
        Some("charly-vibes/whisper")
    );
    assert_eq!(
        table.get("wai").map(String::as_str),
        Some("charly-vibes/wai")
    );
    // The annotated `ddl (this tool)` binary cell parses to `ddl`.
    assert_eq!(
        table.get("ddl").map(String::as_str),
        Some("charly-vibes/dulce-de-leche")
    );
    // openspec's crate cell carries npm annotations; the repo column wins.
    assert_eq!(
        table.get("openspec").map(String::as_str),
        Some("fission-ai/openspec")
    );
}

#[test]
fn missing_managed_block_marker_is_a_loud_parse_error() {
    let broken = FIXTURE_README.replace("<!-- MANAGED-TOOLS:START -->\n", "");
    let err = routing_parity::parse_managed_tools(&broken).unwrap_err();
    assert!(err.contains("MANAGED-TOOLS:START"), "got: {err}");
}

#[test]
fn missing_managed_block_end_marker_is_a_loud_parse_error() {
    let broken = FIXTURE_README.replace("<!-- MANAGED-TOOLS:END -->", "");
    let err = routing_parity::parse_managed_tools(&broken).unwrap_err();
    assert!(err.contains("MANAGED-TOOLS:END"), "got: {err}");
}

#[test]
fn unparseable_list_line_is_a_loud_parse_error() {
    // A list item without the `Tier (N): a, b.` shape must never be
    // silently skipped (anti-goal: no papering over parse failures).
    let broken = FIXTURE_README.replace("- Core (2): wai, testaruda.", "- Core wai testaruda");
    let err = routing_parity::parse_managed_tools(&broken).unwrap_err();
    assert!(err.contains("- Core wai testaruda"), "got: {err}");
}

#[test]
fn declared_count_mismatch_is_a_loud_parse_error() {
    // ddl drift-tests its own block; a count/list disagreement is drift
    // and must fail loudly here too.
    let broken =
        FIXTURE_README.replace("- Core (2): wai, testaruda.", "- Core (3): wai, testaruda.");
    let err = routing_parity::parse_managed_tools(&broken).unwrap_err();
    assert!(err.contains("declares 3"), "got: {err}");
}

#[test]
fn missing_canonical_table_is_a_loud_parse_error() {
    let broken = FIXTURE_README.replace("| binary | crate | repository |", "| x | y |");
    let err = routing_parity::parse_canonical_repos(&broken).unwrap_err();
    assert!(err.contains("canonical"), "got: {err}");
}

#[test]
fn malformed_table_row_is_a_loud_parse_error() {
    // A data row whose cells lack the backticked tokens must fail, not
    // vanish.
    let broken = FIXTURE_README.replace(
        "| `turu` | `whisper-vibes` | `charly-vibes/whisper` |",
        "| turu | whisper-vibes | charly-vibes/whisper |",
    );
    let err = routing_parity::parse_canonical_repos(&broken).unwrap_err();
    assert!(err.contains("turu"), "got: {err}");
}

// ---------------------------------------------------------------------------
// Step 2: the resolver — canonical table → binary-name fallback →
// explicit routing.toml override (spec: Repo resolution fallback).
// ---------------------------------------------------------------------------

#[test]
fn tool_listed_in_canonical_table_resolves_to_the_tables_repo() {
    // turu is a name-mismatch row: the resolved repo is the table's
    // repository column, not the binary name.
    let canonical = routing_parity::parse_canonical_repos(FIXTURE_README).unwrap();
    assert_eq!(
        routing_parity::resolve_repo("turu", &canonical, &BTreeMap::new()),
        routing_parity::ResolvedRepo::Canonical("charly-vibes/whisper".to_owned())
    );
}

#[test]
fn tool_absent_from_table_falls_back_to_the_binary_name() {
    // testaruda is deliberately not in the table (no binary/crate
    // mismatch): the first-party assumption is charly-vibes/<binary>.
    let canonical = routing_parity::parse_canonical_repos(FIXTURE_README).unwrap();
    assert_eq!(
        routing_parity::resolve_repo("testaruda", &canonical, &BTreeMap::new()),
        routing_parity::ResolvedRepo::BinaryFallback("charly-vibes/testaruda".to_owned())
    );
}

#[test]
fn explicit_routing_toml_entry_wins_over_both() {
    // The explicit entry is the override — a local worktree path or a
    // fork beats the canonical table and the binary-name fallback.
    let canonical = routing_parity::parse_canonical_repos(FIXTURE_README).unwrap();
    let mut routing = BTreeMap::new();
    routing.insert(
        "turu".to_owned(),
        routing_parity::RoutingEntry {
            repo: "../my-whisper-worktree".to_owned(),
            upstream: false,
        },
    );
    assert_eq!(
        routing_parity::resolve_repo("turu", &canonical, &routing),
        routing_parity::ResolvedRepo::Override("../my-whisper-worktree".to_owned())
    );
}

// ---------------------------------------------------------------------------
// Step 3: the gate — temp routing.toml fixtures; missing entry fails
// naming the tool, unresolved repo path fails, absent ddl sibling
// degrades to a warning with exit 0.
// ---------------------------------------------------------------------------

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Serialize per-test sandbox dirs even under `cargo test` parallelism.
fn fixture_counter() -> u64 {
    static COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
    let mut c = COUNTER.get_or_init(|| Mutex::new(0)).lock().unwrap();
    *c += 1;
    *c
}

/// Fresh unique temp dir, deleted on drop.
struct Sandbox(PathBuf);

impl Sandbox {
    fn fresh(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "evallerina-parity-{}-{}-{}",
            tag,
            std::process::id(),
            fixture_counter(),
        ));
        std::fs::create_dir_all(&path).expect("create sandbox");
        Sandbox(path)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A complete routing.toml for the fixture managed set: first-party
/// siblings exist as dirs inside the sandbox, upstreams carry the flag.
fn full_fixture_toml() -> String {
    format!(
        "[wai]\nrepo = \"{}\"\n\
         [testaruda]\nrepo = \"{}\"\n\
         [turu]\nrepo = \"{}\"\n\
         [bd]\nrepo = \"gastownhall/beads\"\nupstream = true\n\
         [openspec]\nrepo = \"fission-ai/openspec\"\nupstream = true\n",
        "wai-sib-wai", "wai-sib-testaruda", "wai-sib-whisper",
    )
}

/// Sandbox with the three first-party sibling dirs the fixture toml
/// points at.
fn sandbox_with_siblings(tag: &str) -> Sandbox {
    let sb = Sandbox::fresh(tag);
    for dir in ["wai-sib-wai", "wai-sib-testaruda", "wai-sib-whisper"] {
        std::fs::create_dir_all(sb.0.join(dir)).expect("create sibling dir");
    }
    sb
}

fn failing_rows(report: &routing_parity::ParityReport) -> Vec<&routing_parity::ParityRow> {
    report
        .rows
        .iter()
        .filter(|r| r.status == routing_parity::RowStatus::Fail)
        .collect()
}

#[test]
fn managed_tool_missing_from_routing_toml_fails_naming_it() {
    let sb = sandbox_with_siblings("missing-entry");
    // Drop testaruda's entry entirely.
    let toml = full_fixture_toml().replace("[testaruda]\nrepo = \"wai-sib-testaruda\"\n", "");
    let report =
        routing_parity::routing_parity(Some(FIXTURE_README), &toml, &sb.0).expect("gate runs");
    let fails = failing_rows(&report);
    assert_eq!(fails.len(), 1, "expected exactly one failure: {fails:?}");
    assert!(fails[0].tool == "testaruda", "got: {:?}", fails[0]);
    assert!(
        fails[0].detail.contains("ERR_ROUTING_PARITY"),
        "got: {:?}",
        fails[0]
    );
    assert!(
        fails[0].detail.contains("unmapped first-party tool"),
        "got: {:?}",
        fails[0]
    );
}

#[test]
fn unmapped_upstream_tool_fails_as_missing_upstream_entry() {
    let sb = sandbox_with_siblings("missing-upstream");
    // openspec's third-party-ness is visible in the canonical table
    // (fission-ai/openspec), so a missing entry is detectable as a
    // missing upstream entry. (bd's third-party-ness lives only in its
    // own entry flag, so a missing [bd] fails as an unmapped tool —
    // still a failure naming bd, which is the drift signal.)
    let toml = full_fixture_toml().replace(
        "[openspec]\nrepo = \"fission-ai/openspec\"\nupstream = true\n",
        "",
    );
    let report =
        routing_parity::routing_parity(Some(FIXTURE_README), &toml, &sb.0).expect("gate runs");
    let fails = failing_rows(&report);
    assert_eq!(fails.len(), 1, "expected exactly one failure: {fails:?}");
    assert_eq!(fails[0].tool, "openspec");
    assert!(
        fails[0].detail.contains("upstream entry missing"),
        "got: {:?}",
        fails[0]
    );
}

#[test]
fn bd_without_entry_still_fails_naming_bd() {
    // Anti-goal: the gate must not silently skip bd just because its
    // upstream-ness is only declared in its own entry.
    let sb = sandbox_with_siblings("missing-bd");
    let toml =
        full_fixture_toml().replace("[bd]\nrepo = \"gastownhall/beads\"\nupstream = true\n", "");
    let report =
        routing_parity::routing_parity(Some(FIXTURE_README), &toml, &sb.0).expect("gate runs");
    let fails = failing_rows(&report);
    assert_eq!(fails.len(), 1, "expected exactly one failure: {fails:?}");
    assert_eq!(fails[0].tool, "bd");
    assert!(
        fails[0].detail.contains("ERR_ROUTING_PARITY"),
        "got: {:?}",
        fails[0]
    );
}

#[test]
fn unresolved_repo_path_fails_naming_tool_and_path() {
    let sb = Sandbox::fresh("unresolved");
    // No sibling dirs exist in this sandbox: every first-party entry is
    // an unresolved path (spec: Stale repo path detected).
    let toml = full_fixture_toml();
    let report =
        routing_parity::routing_parity(Some(FIXTURE_README), &toml, &sb.0).expect("gate runs");
    let fails = failing_rows(&report);
    assert_eq!(
        fails.len(),
        3,
        "wai, testaruda, turu all unresolved: {fails:?}"
    );
    for f in &fails {
        assert!(f.detail.contains("ERR_ROUTING_PARITY"), "got: {:?}", f);
        assert!(f.detail.contains("does not resolve"), "got: {:?}", f);
    }
    assert!(
        fails.iter().any(|f| f.detail.contains("wai-sib-whisper")),
        "bad path named: {fails:?}"
    );
}

#[test]
fn resolved_paths_pass_with_ok_rows() {
    let sb = sandbox_with_siblings("resolved");
    let report = routing_parity::routing_parity(Some(FIXTURE_README), &full_fixture_toml(), &sb.0)
        .expect("gate runs");
    assert!(
        failing_rows(&report).is_empty(),
        "no failures expected: {report:?}"
    );
    assert_eq!(report.rows.len(), 5, "one row per managed tool");
    let turu = report.rows.iter().find(|r| r.tool == "turu").unwrap();
    assert_eq!(turu.status, routing_parity::RowStatus::Ok);
    assert!(
        turu.detail.contains("charly-vibes/whisper"),
        "canonical resolution surfaced in the row: {turu:?}"
    );
}

#[test]
fn absent_ddl_sibling_degrades_to_warning_exit_ok() {
    let sb = sandbox_with_siblings("no-sibling");
    // ddl_readme = None: standalone clone. The gate degrades to a
    // warning row, nothing fails.
    let report =
        routing_parity::routing_parity(None, &full_fixture_toml(), &sb.0).expect("gate runs");
    assert!(report.degraded);
    assert!(failing_rows(&report).is_empty());
    assert_eq!(report.rows[0].status, routing_parity::RowStatus::Warn);
    assert!(
        report.rows[0].detail.contains("dulce-de-leche"),
        "warning names the absent sibling: {:?}",
        report.rows[0]
    );
}

#[test]
fn routing_toml_parse_failure_is_loud_even_when_sibling_absent() {
    // Anti-goal: warning degradation is ONLY for the absent sibling —
    // never a cover for a broken routing.toml.
    let sb = sandbox_with_siblings("bad-toml");
    let err = routing_parity::routing_parity(None, "not [ valid toml", &sb.0).unwrap_err();
    assert!(err.contains("routing.toml parse error"), "got: {err}");
}

#[test]
fn managed_block_parse_failure_is_loud_never_a_warning() {
    // Anti-goal: the gate must never paper over a parse failure by
    // degrading — only the genuinely-absent sibling may warn.
    let sb = sandbox_with_siblings("bad-block");
    let broken_readme = FIXTURE_README.replace("<!-- MANAGED-TOOLS:START -->\n", "");
    let err = routing_parity::routing_parity(Some(&broken_readme), &full_fixture_toml(), &sb.0)
        .unwrap_err();
    assert!(err.contains("MANAGED-TOOLS:START"), "got: {err}");
}

#[test]
fn broken_routing_entry_is_a_loud_error() {
    let sb = sandbox_with_siblings("bad-entry");
    let toml =
        full_fixture_toml().replace("[wai]\nrepo = \"wai-sib-wai\"", "[wai]\nupstream = true");
    let err = routing_parity::routing_parity(Some(FIXTURE_README), &toml, &sb.0).unwrap_err();
    assert!(err.contains("[wai] lacks a `repo` string"), "got: {err}");
}

// ---------------------------------------------------------------------------
// Step 4: the upstream policy — bd/openspec resolve and carry
// `upstream = true`; the flag is the explicit override that wins over
// bd's first-party-looking fallback identity. Data only: no path
// check, no filing (e4x owns filing semantics).
// ---------------------------------------------------------------------------

#[test]
fn bd_resolves_and_carries_upstream_true() {
    let sb = sandbox_with_siblings("bd-upstream");
    // bd is NOT in the canonical table; its fallback identity would
    // look first-party (charly-vibes/bd), but the explicit
    // `upstream = true` flag wins.
    let report = routing_parity::routing_parity(Some(FIXTURE_README), &full_fixture_toml(), &sb.0)
        .expect("gate runs");
    let bd = report.rows.iter().find(|r| r.tool == "bd").unwrap();
    assert_eq!(bd.status, routing_parity::RowStatus::Ok, "{bd:?}");
    assert!(bd.detail.contains("upstream"), "{bd:?}");
    assert!(bd.detail.contains("gastownhall/beads"), "{bd:?}");
}

#[test]
fn openspec_resolves_and_carries_upstream_true() {
    let sb = sandbox_with_siblings("openspec-upstream");
    let report = routing_parity::routing_parity(Some(FIXTURE_README), &full_fixture_toml(), &sb.0)
        .expect("gate runs");
    let row = report.rows.iter().find(|r| r.tool == "openspec").unwrap();
    assert_eq!(row.status, routing_parity::RowStatus::Ok, "{row:?}");
    assert!(row.detail.contains("fission-ai/openspec"), "{row:?}");
}

#[test]
fn third_party_identity_without_upstream_flag_fails() {
    let sb = sandbox_with_siblings("openspec-no-flag");
    let toml = full_fixture_toml().replace(
        "[openspec]\nrepo = \"fission-ai/openspec\"\nupstream = true",
        "[openspec]\nrepo = \"fission-ai/openspec\"",
    );
    let report =
        routing_parity::routing_parity(Some(FIXTURE_README), &toml, &sb.0).expect("gate runs");
    let fails = failing_rows(&report);
    assert_eq!(fails.len(), 1, "{fails:?}");
    assert_eq!(fails[0].tool, "openspec");
    assert!(
        fails[0].detail.contains("must carry upstream = true"),
        "{:?}",
        fails[0]
    );
}

#[test]
fn upstream_entries_are_never_path_checked() {
    // Data-only: bd's `repo` value is a slug, not a local checkout —
    // the gate must not require it to exist (upstream repos are never
    // filed in, so there is nothing to resolve).
    let sb = Sandbox::fresh("upstream-no-path");
    // No sibling dirs at all: bd/openspec still come out OK, only the
    // first-party entries fail on unresolved paths.
    let report = routing_parity::routing_parity(Some(FIXTURE_README), &full_fixture_toml(), &sb.0)
        .expect("gate runs");
    for tool in ["bd", "openspec"] {
        let row = report.rows.iter().find(|r| r.tool == tool).unwrap();
        assert_eq!(row.status, routing_parity::RowStatus::Ok, "{row:?}");
    }
}
