//! Purpose: tier-0 routing-map parity gate (evallerina-0sy, openspec
//! add-ecosystem-doc-evals, design D2) — `routing.toml` must mirror
//! dulce-de-leche's MANAGED-TOOLS block so every managed tool's faults
//! have a ticket-routing target.
//! Responsibilities: parse the ddl sibling README (`../dulce-de-leche/
//! README.md`) for the MANAGED-TOOLS tool list and the canonical-names
//! table (binary → repository; the table deliberately lists only
//! binary/crate name mismatches), resolve each managed tool's repo
//! reference (canonical table → binary-name fallback → explicit
//! routing.toml override), parse routing.toml entries with their
//! `upstream` flag (data-only: third-party upstreams `bd` and
//! `openspec` are mapped but never filed — filing semantics belong to
//! e4x), and diff the two maps into per-tool parity rows. Failure rows
//! carry `ERR_ROUTING_PARITY`; the only warning path is a genuinely
//! absent ddl sibling (standalone clones degrade, exit 0).
//! Rationale: ddl's managed block is the ecosystem source of truth
//! (drift-tested inside ddl itself); a tier-0 gate here fails `just
//! tier0` the moment routing.toml drifts, instead of letting tool
//! faults route to nowhere. Parse failures are always loud errors —
//! the gate must never paper over a broken managed block (anti-goal),
//! and warning degradation is reserved exclusively for the absent
//! sibling checkout. Row statuses are explicit: `ok`, `warn` (named,
//! never silent), `fail`.

use std::collections::BTreeMap;

/// Path of the ddl sibling README, relative to the evallerina repo
/// root — the MANAGED-TOOLS source of truth (design D2).
pub const DDL_README: &str = "../dulce-de-leche/README.md";

const MANAGED_START: &str = "<!-- MANAGED-TOOLS:START -->";
const MANAGED_END: &str = "<!-- MANAGED-TOOLS:END -->";

/// One parsed routing.toml `[tool]` entry: the repo reference plus the
/// `upstream` flag. `upstream = true` is DATA only (openspec
/// add-ecosystem-doc-evals): third-party upstreams (`bd`, `openspec`)
/// must be mapped so parity is complete, but they are never filed in —
/// the filing-side semantics (`skipped`/`unmapped_upstream`) belong to
/// e4x and are out of scope here.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingEntry {
    pub repo: String,
    pub upstream: bool,
}

/// The resolved repo reference for a managed tool — the spec's
/// resolution chain made explicit so every branch is testable:
/// explicit routing.toml entry (override wins) → ddl canonical-names
/// table → binary-name fallback (`charly-vibes/<binary>`).
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedRepo {
    /// Explicit routing.toml entry — the override, always wins.
    Override(String),
    /// ddl's canonical-names table (name-mismatch rows).
    Canonical(String),
    /// Not in the table: first-party assumption `charly-vibes/<binary>`.
    BinaryFallback(String),
}

impl ResolvedRepo {
    /// The reference string behind the resolution.
    pub fn reference(&self) -> &str {
        match self {
            ResolvedRepo::Override(r)
            | ResolvedRepo::Canonical(r)
            | ResolvedRepo::BinaryFallback(r) => r,
        }
    }
}

/// Resolve a managed tool's repo reference per the spec chain:
/// explicit routing.toml entry (override wins) → ddl canonical-names
/// table → binary-name fallback (`charly-vibes/<binary>`).
pub fn resolve_repo(
    tool: &str,
    canonical: &BTreeMap<String, String>,
    routing: &BTreeMap<String, RoutingEntry>,
) -> ResolvedRepo {
    if let Some(entry) = routing.get(tool) {
        return ResolvedRepo::Override(entry.repo.clone());
    }
    if let Some(slug) = canonical.get(tool) {
        return ResolvedRepo::Canonical(slug.clone());
    }
    ResolvedRepo::BinaryFallback(format!("charly-vibes/{tool}"))
}

/// Parse routing.toml content into `[tool]` → [`RoutingEntry`]. Every
/// entry must carry a non-empty `repo` string; `upstream` is optional
/// and must be a boolean when present. Structure mirrors
/// [`crate::routing::parse_routing_toml`] (tool → repo strings) with
/// the additive `upstream` flag; parse failures are loud errors.
pub fn parse_routing_entries(content: &str) -> Result<BTreeMap<String, RoutingEntry>, String> {
    let value: toml::Value =
        toml::from_str(content).map_err(|e| format!("routing.toml parse error: {e}"))?;
    let table = value
        .as_table()
        .ok_or("routing.toml must be a table of [tool] sections")?;
    let mut map = BTreeMap::new();
    for (tool, item) in table {
        let repo = item
            .get("repo")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("routing.toml [{tool}] lacks a `repo` string"))?;
        if repo.trim().is_empty() {
            return Err(format!("routing.toml [{tool}] has an empty `repo` path"));
        }
        let upstream = match item.get("upstream") {
            None => false,
            Some(v) => v
                .as_bool()
                .ok_or_else(|| format!("routing.toml [{tool}] `upstream` must be a boolean"))?,
        };
        map.insert(
            tool.clone(),
            RoutingEntry {
                repo: repo.to_owned(),
                upstream,
            },
        );
    }
    Ok(map)
}

/// Extract the managed tool list from the ddl README's MANAGED-TOOLS
/// block. List items have the shape `- <Tier> (N): a, b, c.` — the
/// declared count must match the parsed list (ddl drift-tests its own
/// block; a disagreement here is drift). Prose lines inside the block
/// are tolerated; list items that do not match the shape are loud
/// parse errors, never silently skipped.
pub fn parse_managed_tools(readme: &str) -> Result<Vec<String>, String> {
    let start = readme
        .find(MANAGED_START)
        .ok_or("ddl README: MANAGED-TOOLS:START marker not found")?;
    let body = &readme[start + MANAGED_START.len()..];
    let end = body
        .find(MANAGED_END)
        .ok_or("ddl README: MANAGED-TOOLS:END marker not found")?;
    let block = &body[..end];

    let mut tools = Vec::new();
    for line in block.lines() {
        let line = line.trim();
        if !line.starts_with("- ") {
            continue; // prose, not a registry list item
        }
        let rest = &line[2..];
        let colon = rest
            .find(": ")
            .ok_or_else(|| format!("ddl README: unparseable managed-tools list line: {line:?}"))?;
        let (count_part, list) = rest.split_at(colon);
        let list = &list[2..];
        let count_open = count_part
            .rfind('(')
            .ok_or_else(|| format!("ddl README: unparseable managed-tools list line: {line:?}"))?;
        let count: usize = count_part[count_open + 1..]
            .trim_end_matches(')')
            .trim()
            .parse()
            .map_err(|_| format!("ddl README: unparseable managed-tools list line: {line:?}"))?;

        let mut tier_tools = Vec::new();
        for token in list.split(',') {
            let token = token.trim().trim_end_matches('.');
            if token.is_empty() {
                return Err(format!(
                    "ddl README: empty tool name in managed-tools list line: {line:?}"
                ));
            }
            tier_tools.push(token.to_owned());
        }
        if tier_tools.len() != count {
            return Err(format!(
                "ddl README: managed-tools list line {line:?} declares {count} tools but lists {}",
                tier_tools.len()
            ));
        }
        tools.extend(tier_tools);
    }
    if tools.is_empty() {
        return Err("ddl README: MANAGED-TOOLS block lists no tools".to_owned());
    }
    Ok(tools)
}

/// Extract the first backtick-quoted token from a table cell (e.g.
/// `` `ddl` (this tool) `` → `ddl`; the annotations ride along outside
/// the backticks).
fn first_backtick_token(cell: &str) -> Option<String> {
    let open = cell.find('`')?;
    let rest = &cell[open + 1..];
    let close = rest.find('`')?;
    let token = &rest[..close];
    if token.is_empty() {
        return None;
    }
    Some(token.to_owned())
}

/// Status of one parity row. `warn` is always named and explicit —
/// never a silent skip.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowStatus {
    Ok,
    Warn,
    Fail,
}

/// One per-tool parity row: the verdict for a managed tool.
#[derive(Debug, Clone, PartialEq)]
pub struct ParityRow {
    pub tool: String,
    pub status: RowStatus,
    pub detail: String,
}

/// The full gate result. `degraded = true` means the ddl sibling was
/// absent and parity was not enforced (standalone clones pass tier-0
/// with a warning, per design D2).
#[derive(Debug, Clone, PartialEq)]
pub struct ParityReport {
    pub degraded: bool,
    pub rows: Vec<ParityRow>,
}

impl ParityReport {
    /// True when any row failed — the caller's exit-code signal.
    pub fn failed(&self) -> bool {
        self.rows.iter().any(|r| r.status == RowStatus::Fail)
    }

    /// Machine-checkable JSON shape (one object per row), so CI output
    /// is reviewable without reading code.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "degraded": self.degraded,
            "rows": self.rows.iter().map(|r| serde_json::json!({
                "tool": r.tool,
                "status": match r.status {
                    RowStatus::Ok => "ok",
                    RowStatus::Warn => "warn",
                    RowStatus::Fail => "fail",
                },
                "detail": r.detail,
            })).collect::<Vec<_>>(),
        })
    }
}

/// The tier-0 routing-map parity gate. Inputs are injectable so tests
/// can drive every branch without a real workspace: `ddl_readme` is
/// the ddl sibling README's content (`None` when the checkout is
/// absent), `routing_toml` the routing.toml content, `root` the
/// evallerina repo root for relative-path resolution.
///
/// Checks per managed tool (design D2 + spec scenarios):
/// - no routing.toml entry → FAIL `ERR_ROUTING_PARITY` naming the tool
///   (`unmapped first-party tool` / `upstream entry missing`),
/// - third-party identity (canonical table) without `upstream = true`
///   → FAIL naming the tool,
/// - `upstream = true` → OK, data only (never path-checked, never
///   filed — e4x owns filing semantics),
/// - first-party entry → repo path must resolve from the root, else
///   FAIL naming the tool and the bad path.
///
/// Parse failures (managed block, canonical table, routing.toml) are
/// always loud errors. The ONLY warning path is the genuinely absent
/// ddl sibling: the report degrades with an explicit warn row and the
/// caller exits 0 (anti-goal: never paper over parse failures, never
/// silently skip tools).
pub fn routing_parity(
    ddl_readme: Option<&str>,
    routing_toml: &str,
    root: &std::path::Path,
) -> Result<ParityReport, String> {
    // Parse routing.toml FIRST — a broken map is a loud error even in
    // degraded mode (anti-goal: warning degradation never covers a
    // parse failure).
    let entries = parse_routing_entries(routing_toml)?;

    let Some(readme) = ddl_readme else {
        return Ok(ParityReport {
            degraded: true,
            rows: vec![ParityRow {
                tool: "*".to_owned(),
                status: RowStatus::Warn,
                detail: format!(
                    "ddl sibling checkout absent ({}) — routing parity not enforced (standalone clone; CI must check out the sibling)",
                    DDL_README
                ),
            }],
        });
    };

    // Source-of-truth parses are loud errors, never degradations.
    let tools = parse_managed_tools(readme)?;
    let canonical = parse_canonical_repos(readme)?;

    let mut rows = Vec::new();
    for tool in tools {
        let entry = entries.get(&tool);
        let identity = canonical
            .get(&tool)
            .cloned()
            .unwrap_or_else(|| format!("charly-vibes/{tool}"));
        let third_party_identity = !identity.starts_with("charly-vibes/");

        let Some(e) = entry else {
            let kind = if third_party_identity {
                format!(
                    "upstream entry missing for '{tool}' (resolves to {identity}) — upstream tools are data-only but must be mapped"
                )
            } else {
                format!(
                    "unmapped first-party tool '{tool}' (resolves to {identity}) — no routing.toml entry"
                )
            };
            rows.push(ParityRow {
                tool,
                status: RowStatus::Fail,
                detail: format!("ERR_ROUTING_PARITY: {kind}"),
            });
            continue;
        };

        // The explicit upstream flag is the override and wins: bd's
        // fallback identity would look first-party, but the entry's
        // flag marks it third-party (design D2). Data only — never
        // path-checked, never filed (filing semantics: e4x).
        if e.upstream {
            rows.push(ParityRow {
                tool,
                status: RowStatus::Ok,
                detail: format!("upstream entry ok (never filed): {}", e.repo),
            });
            continue;
        }
        if third_party_identity {
            rows.push(ParityRow {
                detail: format!(
                    "ERR_ROUTING_PARITY: third-party upstream '{tool}' ({identity}) must carry upstream = true"
                ),
                tool,
                status: RowStatus::Fail,
            });
            continue;
        }

        // First-party: the repo path must resolve from the repo root.
        let resolved = crate::routing::resolve_repo(&e.repo, root);
        let (status, detail) = if std::path::Path::new(&resolved).exists() {
            (
                RowStatus::Ok,
                format!("repo '{resolved}' resolves (identity: {identity})"),
            )
        } else {
            (
                RowStatus::Fail,
                format!(
                    "ERR_ROUTING_PARITY: repo path '{resolved}' for '{tool}' does not resolve from the repo root"
                ),
            )
        };
        rows.push(ParityRow {
            detail,
            tool,
            status,
        });
    }
    Ok(ParityReport {
        degraded: false,
        rows,
    })
}

/// Parse the canonical-names table (header `| binary | crate |
/// repository |`) from the ddl README into binary name → repository
/// slug. The table deliberately lists only binary/crate name
/// mismatches; absence from this map is not an error (the resolver
/// falls back to the binary name). Malformed data rows are loud parse
/// errors — a half-parseable source of truth is worse than none.
pub fn parse_canonical_repos(readme: &str) -> Result<BTreeMap<String, String>, String> {
    let header_idx = readme.lines().position(|line| {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        cells.len() >= 5 && cells[1] == "binary" && cells[2] == "crate" && cells[3] == "repository"
    });
    let Some(header_idx) = header_idx else {
        return Err(
            "ddl README: canonical-names table (binary | crate | repository) not found".to_owned(),
        );
    };

    let mut map = BTreeMap::new();
    for line in readme.lines().skip(header_idx + 1) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 5 {
            break; // table ended (blank line or prose)
        }
        if cells[1].starts_with("---") {
            continue; // markdown separator row
        }
        let binary = first_backtick_token(cells[1]).ok_or_else(|| {
            format!("ddl README: canonical-table row lacks a backticked binary name: {line:?}")
        })?;
        let repo = first_backtick_token(cells[3]).ok_or_else(|| {
            format!("ddl README: canonical-table row for '{binary}' lacks a backticked repository: {line:?}")
        })?;
        map.insert(binary, repo);
    }
    Ok(map)
}
