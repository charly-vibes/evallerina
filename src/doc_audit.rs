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

use std::collections::BTreeSet;
use std::process::Command;

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
