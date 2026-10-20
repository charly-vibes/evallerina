//! Purpose: tier-0 doc-coverage audit (evallerina-fok, openspec
//! add-ecosystem-doc-evals design D1/D4) — harvest each tool's CLI
//! surface from its `--help` output, scan the tool's documentation
//! surfaces into per-surface feature-token sets, and diff the two into
//! audit rows (`ERR_FEATURE_UNDOCUMENTED`, `ERR_DOC_NOT_FINDABLE`).
//! Responsibilities: run a tool binary with `--help` and extract
//! subcommands, long flags, and documented exit codes; normalize tokens
//! so kebab/snake/spacing variants are equivalent (substring-normalized
//! matching per D1); scan README.md, docs/**, llms.txt, and AGENTS.md
//! managed blocks; treat llms.txt + managed blocks as breadcrumb
//! sources for reachability; carry provenance for every non-extracted
//! source (`skip` for `surface = "none"`, `binary_absent` for a missing
//! binary, `repo_absent` for a missing local checkout) — never a silent
//! skip; and expose report-shaped rows for the existing dashboard.
//! Rationale: docs drift silently; this gate makes undocumented or
//! unfindable features visible in `just tier0` output the moment they
//! drift. Normalization rules are general (lowercase, alphanumeric-only
//! canonical form) and never tuned to pass evallerina's own docs; there
//! is deliberately no aggregate doc-quality score — per-token rows only.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// The docs surfaces the inventory scanner reads, relative to the
/// audited repo root: the README, every `docs/**/*.md`, `llms.txt`,
/// and `AGENTS.md` (managed blocks only matter for breadcrumbs, but
/// the whole file is a surface).
const README_SURFACE: &str = "README.md";
const LLMS_SURFACE: &str = "llms.txt";
const AGENTS_SURFACE: &str = "AGENTS.md";
const DOCS_DIR: &str = "docs";

/// Parse `doc_audit.toml` contents into per-tool configs. `binary`
/// defaults to the tool name; `surface = "none"` marks the tool as
/// binary-probe-skippable. Unknown fields are ignored (additive
/// evolution) but a malformed value is a loud parse error.
pub fn parse_config(toml_src: &str) -> Result<Vec<ToolConfig>, String> {
    let value: toml::Value =
        toml::from_str(toml_src).map_err(|e| format!("doc_audit.toml parse error: {e}"))?;
    let table = value
        .as_table()
        .ok_or("doc_audit.toml must be a table of [tool] sections")?;
    let mut configs = Vec::new();
    for (tool, item) in table {
        let repo = item
            .get("repo")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("doc_audit.toml [{tool}] lacks a `repo` string"))?;
        let surface = item
            .get("surface")
            .and_then(toml::Value::as_str)
            .unwrap_or("binary");
        if surface != "none" && surface != "binary" {
            return Err(format!(
                "doc_audit.toml [{tool}]: unknown surface '{surface}'"
            ));
        }
        configs.push(ToolConfig {
            tool: tool.clone(),
            repo: repo.to_owned(),
            binary: Some(
                item.get("binary")
                    .and_then(toml::Value::as_str)
                    .unwrap_or(tool)
                    .to_owned(),
            ),
            surface_none: surface == "none",
        });
    }
    Ok(configs)
}

/// Shape one tool's audit rows as a report_version 1 report row (the
/// additive wiring): a `doc-audit` scenario probing the `docs` channel,
/// one failed check per audit row carrying the check-code. The existing
/// dashboard aggregates these without modification.
pub fn report_row(tool: &str, rows: &[AuditRow]) -> serde_json::Value {
    let checks: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": format!("doc:{}", r.token),
                "passed": false,
                "code": r.code,
                "reason": format!("feature token '{}' is undocumented or unfindable", r.token),
            })
        })
        .collect();
    serde_json::json!({
        "report_version": 1,
        "scenario": "doc-audit",
        "tool": tool,
        "status": "failed",
        "checks": checks,
    })
}

/// Per-tool audit configuration, parsed from the checked-in
/// `doc_audit.toml` (`[tool]` sections, same shape as routing.toml).
/// `surface_none` marks non-Rust-CLI tools (e.g. `bd`, `openspec`):
/// they skip the binary probe and are recorded in provenance, never
/// silently skipped.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolConfig {
    pub tool: String,
    pub repo: String,
    pub binary: Option<String>,
    pub surface_none: bool,
}

/// Provenance for one tool's audit run: the loud record of what the
/// audit actually did. `extracted` carries the harvested token set;
/// `skip` (surface none), `binary_absent`, and `repo_absent` are
/// explicit non-extractions — never silent (anti-goal, pinned by test).
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    pub tool: String,
    pub status: String,
    pub detail: String,
    pub tokens: BTreeSet<String>,
}

/// One repo's doc inventory: a token set per scanned surface, plus the
/// breadcrumb set — the tokens reachable from `llms.txt` or AGENTS.md
/// managed blocks (the discovery channels a fresh agent actually
/// follows). Design D4: reachability is measured against breadcrumbs,
/// not against raw doc presence.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Inventory {
    /// Surface path (relative to the repo root) → canonical tokens.
    pub surfaces: BTreeMap<String, BTreeSet<String>>,
    /// Tokens reachable from `llms.txt` or managed-block breadcrumbs.
    pub breadcrumbs: BTreeSet<String>,
}

/// Scan a repo's documentation surfaces into per-surface token sets
/// and the breadcrumb set. Missing surfaces are simply absent from the
/// map (a repo without llms.txt has no breadcrumb channel — that
/// itself makes its features unfindable, which the reachability check
/// reports, not silently ignores).
pub fn scan_inventory(repo: &Path) -> Result<Inventory, String> {
    let mut inv = Inventory::default();
    for (rel, text) in surface_texts(repo)? {
        let tokens = harvest_doc_tokens(&text);
        if rel == LLMS_SURFACE {
            inv.breadcrumbs.extend(tokens.iter().cloned());
        } else if rel == AGENTS_SURFACE {
            let managed = managed_block_text(&text);
            inv.breadcrumbs.extend(harvest_doc_tokens(&managed));
        }
        inv.surfaces.insert(rel, tokens);
    }
    Ok(inv)
}

/// Read all scanned surfaces of a repo: README.md, docs/**/*.md,
/// llms.txt, AGENTS.md — those that exist.
fn surface_texts(repo: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for rel in [README_SURFACE, LLMS_SURFACE, AGENTS_SURFACE] {
        let path = repo.join(rel);
        if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            out.push((rel.to_owned(), text));
        }
    }
    let docs = repo.join(DOCS_DIR);
    if docs.is_dir() {
        collect_docs_md(&docs, DOCS_DIR, &mut out)?;
    }
    Ok(out)
}

/// Recursively collect `*.md` files under docs/, keys relative to the
/// repo root, sorted for deterministic iteration.
fn collect_docs_md(
    dir: &Path,
    rel_prefix: &str,
    out: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let rel = format!("{rel_prefix}/{name}");
        let path = entry.path();
        if path.is_dir() {
            collect_docs_md(&path, &rel, out)?;
        } else if name.ends_with(".md") {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            out.push((rel, text));
        }
    }
    Ok(())
}

/// Extract the concatenated text of every `<!-- X:START --> ... <!--
/// X:END -->` managed block in a markdown file — the only part of
/// AGENTS.md that serves as a breadcrumb channel.
fn managed_block_text(text: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in text.lines() {
        let lower = line.to_lowercase();
        if !in_block {
            if lower.contains(":start -->") {
                in_block = true;
            }
            continue;
        }
        if lower.contains(":end -->") {
            in_block = false;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Harvest candidate feature tokens from documentation text: long
/// flags, backtick code spans, hyphenated identifiers, and plain
/// words, all canonicalized. Deliberately generous (a superset):
/// extra doc-side tokens never hide a real drift, while a missing
/// token is what produces an audit row.
fn harvest_doc_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        for word in line.split_whitespace() {
            // Long flags in prose or backticks: `--flag-name`.
            if let Some(flag) = word.strip_prefix("--") {
                let flag = flag.trim_matches(|c: char| !c.is_ascii_alphanumeric());
                if !flag.is_empty() {
                    tokens.insert(canonical_raw(flag));
                }
            }
            // Strip surrounding code-span backticks and punctuation so
            // `deploy`, and `tear-down`,` both yield their token.
            let bare = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            let mut pieces = bare.split(|c: char| !c.is_ascii_alphanumeric());
            // Hyphenated identifiers (tear-down) are one token; plain
            // words are harvested per piece.
            if bare.contains('-') || bare.contains('_') {
                tokens.insert(canonical_raw(bare));
            } else if let Some(piece) = pieces.next() {
                tokens.insert(canonical_raw(piece));
            }
        }
    }
    tokens
}

/// The two doc-coverage check-codes. `ERR_FEATURE_UNDOCUMENTED`: a
/// surface token absent from every scanned surface. `ERR_DOC_NOT_FINDABLE`:
/// a token documented on some surface but not reachable from the
/// breadcrumb channels (llms.txt / managed blocks).
pub const ERR_FEATURE_UNDOCUMENTED: &str = "ERR_FEATURE_UNDOCUMENTED";
pub const ERR_DOC_NOT_FINDABLE: &str = "ERR_DOC_NOT_FINDABLE";

/// One doc-coverage audit row: a surface feature token that is either
/// undocumented or unfindable. Deliberately per-token — no aggregate
/// doc-quality score exists (anti-goal, pinned by test).
#[derive(Debug, Clone, PartialEq)]
pub struct AuditRow {
    pub token: String,
    pub code: String,
}

/// Diff a tool's surface tokens against a doc inventory: absent from
/// all surfaces → `ERR_FEATURE_UNDOCUMENTED`; present on some surface
/// but unreachable from breadcrumbs → `ERR_DOC_NOT_FINDABLE`; present
/// and reachable → no row.
pub fn audit(surface: &BTreeSet<String>, inv: &Inventory) -> Vec<AuditRow> {
    let documented: BTreeSet<&String> = inv.surfaces.values().flatten().collect();
    surface
        .iter()
        .filter_map(|token| {
            if !documented.contains(token) {
                Some(AuditRow {
                    token: token.clone(),
                    code: ERR_FEATURE_UNDOCUMENTED.to_owned(),
                })
            } else if !inv.breadcrumbs.contains(token) {
                Some(AuditRow {
                    token: token.clone(),
                    code: ERR_DOC_NOT_FINDABLE.to_owned(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Run the surface step for one configured tool: either record the
/// `skip` provenance (`surface = "none"`), or probe the binary and
/// harvest its token set. A missing binary is `binary_absent` provenance
/// — loud, not an error and not a silent skip.
pub fn audit_tool_surface(config: &ToolConfig) -> Result<Provenance, String> {
    if config.surface_none {
        return Ok(Provenance {
            tool: config.tool.clone(),
            status: "skip".to_owned(),
            detail: "surface = none: not a --help-probeable CLI surface".to_owned(),
            tokens: BTreeSet::new(),
        });
    }
    let binary = config.binary.as_deref().unwrap_or(&config.tool).to_owned();
    match extract_surface(&binary) {
        Ok(tokens) => Ok(Provenance {
            tool: config.tool.clone(),
            status: "extracted".to_owned(),
            detail: format!("{binary} --help"),
            tokens,
        }),
        Err(_) => Ok(Provenance {
            tool: config.tool.clone(),
            status: "binary_absent".to_owned(),
            detail: format!("binary '{binary}' not runnable — audit cannot probe its surface"),
            tokens: BTreeSet::new(),
        }),
    }
}

/// Canonical form of a feature token: lowercase, alphanumeric
/// characters only. `--dry-run`, `dry_run`, `Dry Run`, and `dryrun` all
/// canonicalize to `dryrun` — one feature, one token (design D1).
fn canonical_raw(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Public token normalization: kebab/snake/spacing variants of a
/// feature name are equivalent. Exposed for contract tests so the
/// normalization rule itself is pinned, not just its application.
pub fn canonical(s: &str) -> String {
    canonical_raw(s)
}

/// Extract the feature-token set from a tool binary's `--help` output:
/// long flags, subcommand names, and documented exit codes (where the
/// help lists them), all in canonical form. The binary is invoked with
/// `--help` and both stdout and stderr are parsed (some CLIs print
/// usage to stderr even on success).
pub fn extract_surface(binary: &str) -> Result<BTreeSet<String>, String> {
    let out = Command::new(binary)
        .arg("--help")
        .output()
        .map_err(|e| format!("cannot run {binary} --help: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut tokens = harvest_tokens(&stdout);
    tokens.extend(harvest_tokens(&stderr));
    Ok(tokens)
}

/// Harvest feature tokens from help text: long flags, subcommand
/// entries under a Commands/Subcommands header, and documented exit
/// codes. General rules only — no tool-specific special cases.
fn harvest_tokens(help: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut in_commands = false;
    let mut in_exit = false;
    for line in help.lines() {
        let trimmed = line.trim();
        // Long flags: `--flag-name` (strip the dashes, canonicalize).
        for word in trimmed.split_whitespace() {
            if let Some(flag) = word.strip_prefix("--") {
                let flag = flag.split('=').next().unwrap_or(flag);
                if !flag.is_empty() && flag.chars().any(|c| c.is_ascii_alphanumeric()) {
                    tokens.insert(canonical_raw(flag));
                }
            }
        }
        // Subcommand section: lines after a `Commands:`/`Subcommands:`
        // header, until a blank line or non-indented line ends it. The
        // first word of each indented entry is the subcommand name.
        if trimmed == "Commands:" || trimmed == "Subcommands:" {
            in_commands = true;
            continue;
        }
        if in_commands {
            if trimmed.is_empty() || !line.starts_with(char::is_whitespace) {
                in_commands = false;
            } else if let Some(name) = trimmed
                .split_whitespace()
                .next()
                .filter(|n| n.chars().any(|c| c.is_ascii_alphanumeric()))
            {
                tokens.insert(canonical_raw(name));
            }
            continue;
        }
        // Exit-code section: `Exit codes:` / `Exit status:` header,
        // then indented `N  description` lines. Each documented code
        // becomes the token `exitN`.
        let lower = trimmed.to_lowercase();
        if lower.starts_with("exit codes:") || lower.starts_with("exit status:") {
            in_exit = true;
            // Same-line codes (`Exit codes: 0, 2`) are harvested too.
            harvest_exit_words(trimmed, &mut tokens);
            continue;
        }
        if in_exit {
            if trimmed.is_empty() || !line.starts_with(char::is_whitespace) {
                in_exit = false;
            } else {
                harvest_exit_words(trimmed, &mut tokens);
            }
            continue;
        }
        // Inline exit-code mention: `Exits with code 2 on bad input`.
        if let Some(idx) = lower
            .find("exit code")
            .or_else(|| lower.find("exit status"))
        {
            let after = &trimmed[idx + "exit code".len()..];
            if let Some(code) = after
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|first| first.parse::<u32>().ok())
            {
                tokens.insert(format!("exit{code}"));
            }
        }
    }
    tokens
}

/// Harvest leading integer exit codes from one exit-section line
/// (`  0  success` → `exit0`); description words are not tokens.
fn harvest_exit_words(line: &str, tokens: &mut BTreeSet<String>) {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if let Ok(code) = digits.parse::<u32>() {
        tokens.insert(format!("exit{code}"));
    }
}
