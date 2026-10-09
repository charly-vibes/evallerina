# Legacy Monolith archetype — stub (evallerina-sqq)

Purpose: exercise the ddl migrate legacy-config path (`.wai/`, `.dont/`,
`.pretender.toml` → symlinks under `.ddl/`).

Planned injected faults: legacy paths present as real directories instead
of symlinks; a stale `.pretender.toml` with an unsupported schema field.

Planned smoke scenario: `ddl migrate` converts legacy paths; replay
asserts the migration envelope and the symlink topology, never the
self-reported migration KPI.

Status: fixture stub only — Must gate (evallerina-sqq) is covered by
polyglot-monorepo + drifted-ecosystem.
