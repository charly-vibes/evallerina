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
