//! Purpose: Contract tests for the tier-0 doc-coverage audit
//! (evallerina-fok, openspec add-ecosystem-doc-evals design D1/D4).
//! Responsibilities: pin the audit pipeline contract — surface
//! extraction from a tool binary's `--help` (subcommands, long flags,
//! documented exit codes) with kebab/snake/spacing token normalization,
//! `surface: none` and `binary_absent` provenance (never silent skips),
//! inventory scanning of README/docs/llms.txt/AGENTS.md managed blocks
//! into per-surface token sets, and breadcrumb reachability
//! distinguishing ERR_FEATURE_UNDOCUMENTED from ERR_DOC_NOT_FINDABLE.
//! All fixtures are throwaway shell scripts and temp-dir repos — no
//! real ecosystem binaries in the contract tests.
//! Rationale: the audit is the mechanism that fails `just tier0` the
//! moment docs drift; its contract must be hermetic and loud about
//! every skip (anti-goal: silent skipping), with normalization rules
//! that are general, never tuned to evallerina's own docs.

use std::fs;
use std::path::PathBuf;

use evallerina::doc_audit;

/// Fresh unique dir under std::env::temp_dir() (no tempfile dep).
fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "evallerina-doc-audit-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create fixture dir");
    dir
}

// ---------------------------------------------------------------------------
// Step 1: surface extraction from a fixture binary's --help
// ---------------------------------------------------------------------------

#[test]
fn surface_extraction_harvests_subcommands_flags_and_exit_codes() {
    let dir = fresh_dir("surface");
    let bin = dir.join("fakecli");
    fs::write(
        &bin,
        "#!/usr/bin/env bash\nexec cat <<'HELP'\nfakecli 1.0\n\nUsage: fakecli <COMMAND>\n\nCommands:\n  deploy     Push the artifact\n  tear-down  Remove it\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n  --dry-run      Do not touch anything\n  --output_dir   Where results go\n\nExit codes:\n  0  success\n  2  bad input\nHELP\n",
    )
    .expect("write fixture script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    }

    let tokens = doc_audit::extract_surface(bin.to_str().expect("utf8 path"))
        .expect("surface extraction succeeds");

    // Subcommands harvested.
    assert!(tokens.contains("deploy"), "deploy missing: {tokens:?}");
    assert!(tokens.contains("teardown"), "tear-down missing: {tokens:?}");
    // Long flags harvested.
    assert!(tokens.contains("dryrun"), "--dry-run missing: {tokens:?}");
    assert!(
        tokens.contains("outputdir"),
        "--output_dir missing: {tokens:?}"
    );
    // Documented exit codes harvested where --help lists them.
    assert!(tokens.contains("exit0"), "exit 0 missing: {tokens:?}");
    assert!(tokens.contains("exit2"), "exit 2 missing: {tokens:?}");
    // Kebab/snake/spacing variants normalize to one token: the set has
    // a single representation for each feature, and re-canonicalizing
    // any spelling lands on the same token.
    assert_eq!(
        tokens.iter().filter(|t| *t == "teardown").count(),
        1,
        "kebab/snake pair must normalize to one token: {tokens:?}"
    );
    assert_eq!(
        doc_audit::canonical("tear-down"),
        doc_audit::canonical("tear_down")
    );
    assert_eq!(
        doc_audit::canonical("tear down"),
        doc_audit::canonical("teardown")
    );
}

// ---------------------------------------------------------------------------
// Step 2: `surface = "none"` tools skip the binary probe, loudly —
// the tool is recorded in provenance, never silently skipped
// (anti-goal, pinned by test).
// ---------------------------------------------------------------------------

#[test]
fn surface_none_skips_binary_probe_with_provenance() {
    // The binary path does not exist: if the extractor probed it, the
    // run would fail loudly — proving `surface = "none"` performs no
    // binary step at all.
    let provenance = doc_audit::audit_tool_surface(&doc_audit::ToolConfig {
        tool: "openspec".into(),
        repo: "fission-ai/openspec".into(),
        binary: None,
        surface_none: true,
    })
    .expect("surface-none tool is recorded, not an error");
    assert_eq!(provenance.status, "skip");
    assert!(provenance.detail.contains("surface"));
    // No binary probe happened, so there is no token set either.
    assert!(provenance.tokens.is_empty());
}

// ---------------------------------------------------------------------------
// Step 3: inventory scanning — fixture repo (README + docs/x.md +
// llms.txt + AGENTS.md managed block) into per-surface token sets
// ---------------------------------------------------------------------------

#[test]
fn inventory_scanner_separates_surfaces_and_breadcrumbs() {
    let dir = fresh_dir("inventory");
    fs::write(
        dir.join("README.md"),
        "# fakecli\n\nRun `deploy` to push. Use `--dry-run` first.\n",
    )
    .expect("write README");
    fs::create_dir_all(dir.join("docs")).expect("mkdir docs");
    fs::write(
        dir.join("docs/x.md"),
        "# Reference\n\nThe `tear-down` command removes everything.\n",
    )
    .expect("write docs/x.md");
    fs::write(
        dir.join("llms.txt"),
        "fakecli quickstart: deploy, tear-down, --dry-run\n",
    )
    .expect("write llms.txt");
    fs::write(
        dir.join("AGENTS.md"),
        "<!-- WAI:START -->\nmanaged guidance: use `--output_dir` for results\n<!-- WAI:END -->\n",
    )
    .expect("write AGENTS.md");

    let inv = doc_audit::scan_inventory(&dir).expect("inventory scan succeeds");

    let readme = inv
        .surfaces
        .get("README.md")
        .expect("README surface present");
    assert!(readme.contains("deploy"), "README tokens: {readme:?}");
    assert!(readme.contains("dryrun"), "README tokens: {readme:?}");

    let docs = inv
        .surfaces
        .get("docs/x.md")
        .expect("docs/x.md surface present");
    assert!(docs.contains("teardown"), "docs tokens: {docs:?}");

    // Breadcrumb sources: llms.txt + AGENTS.md managed blocks.
    assert!(inv.breadcrumbs.contains("deploy"));
    assert!(inv.breadcrumbs.contains("outputdir"));
    // Non-breadcrumb content must not leak into the breadcrumb set:
    // docs/x.md content is a surface, not a breadcrumb source.
    let managed_only = inv
        .surfaces
        .get("AGENTS.md")
        .expect("AGENTS.md surface present");
    assert!(
        managed_only.contains("outputdir"),
        "AGENTS tokens: {managed_only:?}"
    );
    let llms = inv
        .surfaces
        .get("llms.txt")
        .expect("llms.txt surface present");
    assert!(llms.contains("dryrun"), "llms.txt tokens: {llms:?}");
}

// ---------------------------------------------------------------------------
// Step 4: breadcrumb reachability — token in docs/x.md not reachable
// from llms.txt / managed block → ERR_DOC_NOT_FINDABLE; token absent
// everywhere → ERR_FEATURE_UNDOCUMENTED; reachable token → no row.
// ---------------------------------------------------------------------------

#[test]
fn reachability_distinguishes_not_findable_from_undocumented() {
    let dir = fresh_dir("reach");
    // Fixture surface: three features from --help.
    let bin = dir.join("fakecli");
    fs::write(
        &bin,
        "#!/usr/bin/env bash\nexec cat <<'HELP'\nUsage: fakecli\n\nCommands:\n  findable   Documented and findable\n  orphaned   Documented but unreachable\n  hidden     Absent from all docs\nHELP\n",
    )
    .expect("write fixture script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    }

    fs::write(
        dir.join("README.md"),
        "docs for `orphaned` live in docs/x.md\n",
    )
    .expect("write README");
    fs::create_dir_all(dir.join("docs")).expect("mkdir docs");
    fs::write(dir.join("docs/x.md"), "the `findable` command reference\n")
        .expect("write docs/x.md");
    // Breadcrumbs mention only `findable` (and reference docs/x.md).
    fs::write(dir.join("llms.txt"), "start at docs/x.md; try `findable`\n")
        .expect("write llms.txt");

    let surface = doc_audit::extract_surface(bin.to_str().expect("utf8 path"))
        .expect("surface extraction succeeds");
    let inv = doc_audit::scan_inventory(&dir).expect("inventory scan succeeds");
    let rows = doc_audit::audit(&surface, &inv);

    let findable_rows: Vec<_> = rows.iter().filter(|r| r.token == "findable").collect();
    assert!(
        findable_rows.is_empty(),
        "reachable token must produce no row: {rows:?}"
    );
    let orphaned = rows
        .iter()
        .find(|r| r.token == "orphaned")
        .expect("orphaned token must produce a row");
    assert_eq!(orphaned.code, "ERR_DOC_NOT_FINDABLE");
    let hidden = rows
        .iter()
        .find(|r| r.token == "hidden")
        .expect("hidden token must produce a row");
    assert_eq!(hidden.code, "ERR_FEATURE_UNDOCUMENTED");
    assert_eq!(rows.len(), 2, "exactly the two drift rows: {rows:?}");
}

// ---------------------------------------------------------------------------
// Step 5: a configured binary that is missing → `binary_absent`
// provenance: loud, not a silent skip (anti-goal, pinned by test).
// ---------------------------------------------------------------------------

#[test]
fn binary_absent_records_loud_provenance() {
    let dir = fresh_dir("binary-absent");
    let missing = dir.join("no-such-binary");
    let config = doc_audit::ToolConfig {
        tool: "pretender".into(),
        repo: "../pretender".into(),
        binary: Some(missing.to_string_lossy().into_owned()),
        surface_none: false,
    };
    let provenance = doc_audit::audit_tool_surface(&config)
        .expect("absent binary is provenance, not a loud error");
    assert_eq!(provenance.status, "binary_absent");
    assert!(
        provenance
            .detail
            .contains(&missing.to_string_lossy().to_string()),
        "provenance names the missing binary: {provenance:?}"
    );
    assert!(provenance.tokens.is_empty());
}

// ---------------------------------------------------------------------------
// Dogfood (hermetic slice): scanning evallerina's own surfaces yields a
// non-empty inventory — the extractor shakes out on this repo itself.
// ---------------------------------------------------------------------------

#[test]
fn dogfood_inventory_of_evallerina_surfaces_is_non_empty() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let inv = doc_audit::scan_inventory(&root).expect("scan evallerina itself");
    let readme = inv.surfaces.get("README.md").expect("README surface");
    assert!(
        !readme.is_empty(),
        "evallerina README inventory is non-empty"
    );
    assert!(
        inv.surfaces.keys().any(|k| k.starts_with("docs/")),
        "docs/ surfaces scanned: {:?}",
        inv.surfaces.keys().collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Step 6: config parsing (`doc_audit.toml`) + report wiring — audit
// rows become report_version 1 rows carrying the new check-codes;
// report_version stays 1, additive only.
// ---------------------------------------------------------------------------

#[test]
fn doc_audit_toml_parses_tools_surface_none_and_binary() {
    let toml = r#"
[wai]
repo = "../wai"

[openspec]
repo = "fission-ai/openspec"
surface = "none"

[ah]
repo = "../espectacular"
binary = "ah"
"#;
    let configs = doc_audit::parse_config(toml).expect("config parses");
    let wai = configs.iter().find(|c| c.tool == "wai").expect("wai entry");
    assert_eq!(wai.repo, "../wai");
    assert!(!wai.surface_none);
    assert_eq!(
        wai.binary.as_deref(),
        Some("wai"),
        "binary defaults to the tool name"
    );
    let openspec = configs
        .iter()
        .find(|c| c.tool == "openspec")
        .expect("openspec entry");
    assert!(openspec.surface_none, "surface = none is parsed");
    let ah = configs.iter().find(|c| c.tool == "ah").expect("ah entry");
    assert_eq!(ah.binary.as_deref(), Some("ah"), "explicit binary wins");
}

#[test]
fn audit_rows_flow_into_report_as_version_1_rows() {
    use evallerina::report::{aggregate_rows, scenario_channel};

    // The doc-audit scenario must have a declared output channel so
    // aggregate_rows accepts its rows (additive wiring, no bump).
    assert_eq!(scenario_channel("doc-audit"), Some("docs"));

    let rows = vec![doc_audit::report_row(
        "fakecli",
        &[
            doc_audit::AuditRow {
                token: "orphaned".into(),
                code: "ERR_DOC_NOT_FINDABLE".into(),
            },
            doc_audit::AuditRow {
                token: "hidden".into(),
                code: "ERR_FEATURE_UNDOCUMENTED".into(),
            },
        ],
    )];
    assert_eq!(rows[0]["report_version"], 1, "report_version stays 1");

    let configured = vec!["model-a".to_owned()];
    let dashboard = aggregate_rows(&rows, &configured, &scenario_channel)
        .expect("doc-audit rows aggregate cleanly");
    // Doc-audit rows carry no model attribution: they aggregate as
    // non-tier-2 rows, visible in the routing entries (one per code),
    // recorded without AIX-channel action (no cross-model signal).
    let routing = dashboard["routing"].as_array().expect("routing");
    assert_eq!(routing.len(), 2, "both codes are routing-visible");
    let codes: Vec<&str> = routing
        .iter()
        .map(|c| c["code"].as_str().expect("code str"))
        .collect();
    assert!(codes.contains(&"ERR_FEATURE_UNDOCUMENTED"));
    assert!(codes.contains(&"ERR_DOC_NOT_FINDABLE"));
    assert_eq!(dashboard["report_version"], 1);
}
