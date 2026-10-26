---
tags: [pipeline-run:epic-orchestrator-2026-10-26-ablate-rotation-cells-vvt, pipeline-step:ship]
---

SHIP: vvt implementation 0d768e2 + evidence 5110810 + state c0eb937; bd exported; vvt released to open pending Nov 2 live meter (with 8li/17o); nothing to openspec (t3s split is not an openspec task)

Commands run (verification evidence at ship time):
- just tier0 → exit 0 (run fresh by orchestrator at verify step)
- cargo test --quiet → all binaries "test result: ok. N passed; 0 failed"
- python3 yaml.safe_load(.github/workflows/evals-rotation.yml) → exit 0
- hermetic dry-run ablate → 12 rows arm=full + 12 rows arm=ablated
Verified: full verification command set green at 5110810 (see
verify-evallerina-vvt.md addendum); ship = state bookkeeping only
(non-code), no new code changes in this step.
