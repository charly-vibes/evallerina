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
