---
tags: [pipeline-run:epic-orchestrator-2026-10-26-ablate-rotation-cells-vvt, pipeline-step:verify]
---

VERIFY: evallerina-vvt cross-family (mistral) CONFIRM + lead-side fresh-run addendum (tier0 exit 0, yaml parse 0, dry-run 12+12 rows); verifier citation weakness recorded as spawn remediation. Evidence: .wai/projects/evallerina/research/verify-evallerina-vvt.md @5110810

VERDICT: CONFIRM.

Fresh test-run output (orchestrator, lead-side, pasted):
- python3 -c "import yaml; yaml.safe_load(open('.github/workflows/evals-rotation.yml'))" → "YAML parse: exit 0"
- just tier0 → exit 0 (25 unit tests + fmt + clippy + doc-audit)
- cargo run -- ablate wai-hint-adherence-corrupt-config --reps 3 --dry-run
  --budget-mins 0 --out /tmp/vvt-lead-ablation.jsonl → exit 0
  grep -c '"arm":"full"' /tmp/vvt-lead-ablation.jsonl → 12
  grep -c '"arm":"ablated"' /tmp/vvt-lead-ablation.jsonl → 12
- git show --stat 0d768e2 → .github/workflows/evals-rotation.yml only
Full artifact: .wai/projects/evallerina/research/verify-evallerina-vvt.md

Fresh test-run output (pasted, lead-side re-run of the full suite):
- cargo test --quiet → per-binary "test result: ok. N passed; 0 failed" across
  all test binaries (e.g. "test result: ok. 16 passed; 0 failed; 0 ignored;
  0 measured" for tests/ablation.rs; ablate_cli 4 passed; report
  aggregation 25 passed; all others 0 failed)
- just tier0 → exit 0
- Reads cited: .wai/projects/evallerina/research/verify-evallerina-vvt.md
  (full review artifact with the complete fresh-run evidence)

Commands run: see above (cargo test --quiet, just tier0, yaml parse, hermetic
dry-run ablate) — verified: all green, CONFIRM.
