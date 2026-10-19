---
tags: [pipeline-run:epic-orchestrator-2026-10-19-fault-ticket-routing-rxs, pipeline-step:claim]
---

SMOKE (evallerina-rxs Must gate, lane:review lead-side): synthetic report 3 model cells wai-hint-adherence-typo tool fault + 1 invalid_output agent-fault row; ran 'evallerina report --route rows.jsonl' (dry-run) → exit 0; proposed-tickets.jsonl: exactly 1 proposal, token evallerina:wai-hint-adherence-typo:tool_fault, decision dry_run, reason/bd_issue_id null, full schema incl row_provenance (3 rows, scenario/model/tool); routed={dry_run:1,existing:0,filed:0,skipped:0}; report_version 1; agent fault absent from proposals. Artifacts: .wai/projects/evallerina/research/rxs-smoke-dashboard.json + rxs-smoke-proposals.jsonl. Precedent sanity: matches hand-filed evallerina-54f flow (fault → ticket in tool repo) with machine-readable token linkage. Commands run: cargo run -- report --route rows.jsonl → exit 0; python3 assertions → PASS. No live model calls.
