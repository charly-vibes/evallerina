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
