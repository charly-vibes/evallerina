# reports/

Tier-2 rotation rows (report_version 1 trial rows) and fault dashboards.

Backfill provenance (evallerina-17o): rows recovered from GHA artifacts of the
scheduled runs before artifact expiry —

- `tier2-rotation-2026-10-12.jsonl` — run 38244717029 (72 cells)
- `tier2-rotation-2026-10-19.jsonl` — run 39232183468 (72 cells)

The weekly rotation (evals-rotation.yml) appends future rows to
`reports/tier2-rotation.jsonl` in the runner workspace; persistence of future
rows in-repo is tracked in evallerina-17o.
