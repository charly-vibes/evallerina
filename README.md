# evallerina

> *Evals with an avatar — measuring whether agents actually use the tools.*

[![tracked with wai](https://img.shields.io/badge/tracked%20with-wai-blue)](https://github.com/charly-vibes/wai)

Consumer eval battery for the **dulce-de-leche tool family** (the charly-vibes
CLI suite built on [genesis-vibes](https://github.com/charly-vibes/genesis)).
evallerina answers one question empirically: **do LLM agents read, parse, and act
on the suite's AIX output channels?** — JSON envelopes (`ok`, `warnings`,
`hints`), self-healing suggestions, managed blocks in `AGENTS.md`, state
machines, `llms.txt` discovery.

## Method

Scenarios follow the normative [genesis evals-guidelines
spec](https://github.com/charly-vibes/genesis/tree/main/openspec/specs/evals-guidelines):

- **Sandboxed fixtures** — fresh temp dir, tool binaries pre-provisioned, isolated
  `HOME`, network denied unless the scenario opts in
- **Process-boundary scoring** — assert only on exit codes, captured stdout/stderr
  envelopes, and filesystem state; free-form agent text is never scored
- **Tier ladder** — tier 0 static checks (every push, no model) / tier 1 scripted
  replay of recorded transcripts (nightly, no model) / tier 2 live model runs
  (scheduled rotation, never gates a push)
- **Free-tier entry** — OpenRouter `:free` model ids are the default first
  candidates of the model registry; raw ids recorded verbatim; n ≥ 3 repetitions
  per scenario × model cell; 429 → one retry then `rate_limited`
- **Fault taxonomy** — every failure classified (`ERR_ENVELOPE_HINT_BLINDNESS`,
  `ERR_DOC_DRIFT_BLINDNESS`, `ERR_ACTION_FORMAT_VIOLATION`, …) and routed: tool
  faults become tickets in the tool's repo, agent faults reproducible across ≥ 3
  models point at the output channel (AIX work in genesis)

Key eval families: contrived-failure injection (hint adherence), doc-drift
blindness (stale `AGENTS.md` bait vs live envelope), managed-block boundary
audits, and A/B ablation of the AIX artifacts themselves — the delta is the
measured value of the documentation investment.

## Layout

```
openspec/        # change proposals & specs for the harness itself
src/             # harness crate (depends on genesis-vibes: Fixture, evals module)
scenarios/       # scenario definitions + fixtures (provenance: ticket or corpus note)
reports/         # tier-2 report rows, report_version JSON contract
.github/workflows/  # tier-0 gates + tier-1/2 scheduled rotations
```

## Status

Scaffold phase. Roadmap tracked in beads (`bd ready`).

## Related

- [dulce-de-leche](https://github.com/charly-vibes/dulce-de-leche) — the suite being evaluated
- [genesis evals how-to](https://github.com/charly-vibes/genesis/blob/main/docs/how-to/evals.md)
- [genesis eval-report contract](https://github.com/charly-vibes/genesis/blob/main/docs/reference/eval-report.md)
