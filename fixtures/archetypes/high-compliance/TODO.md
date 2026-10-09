# High-Compliance archetype — stub (evallerina-sqq)

Purpose: license/metadata audit scenarios — README license headers vs
Cargo/package metadata mismatches (the documented inconsistency class).

Planned injected faults: `LICENSE` says Apache-2.0 while `Cargo.toml`
declares a different license; a crate missing the license field.

Planned smoke scenario: metadata audit across the triad languages;
replay asserts the *delta* between claimed and declared licenses is
reported, never hard-asserts tool KPI numbers.

Status: fixture stub only — Must gate (evallerina-sqq) is covered by
polyglot-monorepo + drifted-ecosystem.
