# DevOps/headless archetype — stub (evallerina-sqq)

Purpose: CI-only environment, no interactive hooks — every tool must
degrade to non-interactive envelopes (no TTY prompts, no editor spawns).

Planned injected faults: missing `OPENROUTER_API_KEY`, read-only HOME,
no git remote configured.

Planned smoke scenario: wai/dont/ah invoked headless; replay asserts
every envelope stays parseable and exits carry the documented codes.

Status: fixture stub only — Must gate (evallerina-sqq) is covered by
polyglot-monorepo + drifted-ecosystem.
