# Verification of evallerina-xc3

## Evidence

### Git diff

Commit `dd1352d` only modifies `src/report.rs` and `tests/report_aggregation.rs`:

```diff
commit dd1352df650a467dd5ba73d66ccaa7b1ec319e19
author charly vibes <charly.vibes@proton.me> 1635329693 -0300

date Mon Oct 26 16:01:33 2026 -0300

report: separate invalid_output faults from failed-trial hint-blindness (evallerina-xc3)

EDGE-003: trial-level invalid_output faults (ERR_ACTION_FORMAT_VIOLATION,
empty replays) were folded into the same channel/code space as failed-trial
hint-blindness faults, overstating the envelope-hint signal.

Additive change (dashboard_version/report_version unchanged, no fields
removed, no aggregate score):

- FaultKey::Agent gains a status_class key dimension ("failed" |
"invalid_output"), so the two fault classes produce distinct matrix and
routing entries even in the same channel.
- Agent-fault matrix and routing entries emit the additive status_class
field; tool-fault entries keep the prior shape (no-code contract).
- distinct_models counts models WITHIN one status class (conservative
reading): an invalid_output occurrence can never push a failed-class
finding over the routing threshold.
- Derivation-rule comment (AC-2): hint-blindness counts derive ONLY from
status_class="failed" entries; invalid_output-class faults carry empty
replays by definition and are never blended in.

New RED-first test invalid_output_never_blends_into_failed_hint_blindness
pins the separation; all 24 pre-existing tests pass with no assertion
edits. just tier0 green.

diff --git a/src/report.rs b/src/report.rs
index 5664d8a..9fb5ffa 100644
--- a/src/report.rs
+++ b/src/report.rs
@@ -156,12 +156,21 @@ pub fn scenario_channel(name: &str) -> Option<&'static str> {
 }
 
 /// One accumulator key in the routing maps: agent faults key on
-/// (channel, code), tool faults on (tool, check, channel) because they
-/// carry no code by contract.
+/// (channel, code, status_class), tool faults on (tool, check, channel)
because they carry no code by contract. The `status_class` dimension
+/// (evallerina-xc3) keeps trial-level `invalid_output` faults
+/// (ERR_ACTION_FORMAT_VIOLATION — empty replays) separate from
+/// failed-trial hint-blindness faults in the same channel.
 #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FaultKey {
-    /// Agent fault: the `ERR_*` taxonomy code within one output channel.
-    Agent { channel: String, code: String },
+    /// Agent fault: the `ERR_*` taxonomy code within one output channel,
+    /// from a trial of the given status class ("failed" |
+    /// "invalid_output").
+    Agent {
+        channel: String,
+        code: String,
+        status_class: String,
+    },
     /// Tool fault: no code; routed to the tool's repository.
     Tool {
         tool: String,
@@ -202,7 +211,7 @@ pub fn aggregate_rows(
     }
     let routing_threshold = configured_models.len().min(ROUTING_THRESHOLD);
 
-    let mut matrix_cells: BTreeMap<(String, String, String), FaultTally> = BTreeMap::new();
+    let mut matrix_cells: BTreeMap<(String, String, String, String), FaultTally> = BTreeMap::new();
     let mut routings: BTreeMap<FaultKey, FaultTally> = BTreeMap::new();
     let mut coverage = Coverage::default();
     let mut report_versions = BTreeSet::new();
@@ -271,9 +280,17 @@ pub fn aggregate_rows(
             coverage.checks_failed += 1;
             match check.get("code").and_then(serde_json::Value::as_str) {
                 Some(code) => {
+                    // Derivation rule (AC-2, evallerina-xc3): hint-blindness
+                    // counts derive ONLY from status_class="failed" agent
+                    // faults. The check rows of a failed trial carry rich
+                    // exploratory replays; trial-level invalid_output faults
+                    // (ERR_ACTION_FORMAT_VIOLATION) have empty replays by
+                    // definition and key status_class="invalid_output"
+                    // below — the two are never blended.
                     let key = FaultKey::Agent {
                         channel: channel.to_owned(),
                         code: code.to_owned(),
+                        status_class: status.to_owned(),
                     };
                     tally(
                         &mut routings,
@@ -316,6 +333,7 @@ pub fn aggregate_rows(
             let key = FaultKey::Agent {
                 channel: channel.to_owned(),
                 code: code.to_owned(),
+                status_class: "invalid_output".to_owned(),
             };
             tally(&mut routings, &mut matrix_cells, &key, scenario, model, "");
         }
@@ -323,14 +341,22 @@ pub fn aggregate_rows(
 
     let matrix: Vec<serde_json::Value> = matrix_cells
         .iter()
-        .map(|((channel, code, model), tally)| {
-            json!({
+        .map(|((channel, code, model, status_class), tally)| {
+            let mut cell = json!({
                 "channel": channel,
                 "code": code,
                 "model": model,
                 "occurrences": tally.occurrences,
                 "scenarios": tally.scenarios.iter().collect::<Vec<_>>(),
-            })
+            });
+            // Additive (evallerina-xc3): agent-fault cells carry the
+            // trial status class the fault came from; tool-fault cells
+            // (empty status_class by the no-code contract) keep the
+            // prior shape.
+            if !status_class.is_empty() {
+                cell["status_class"] = json!(status_class);
+            }
+            cell
         })
         .collect();
 
@@ -339,11 +365,28 @@ pub fn aggregate_rows(
         .map(|(key, tally)| {
             let distinct_models = tally.models.len();
             match key {
-                FaultKey::Agent { channel, code } => {
+                FaultKey::Agent {
+                    channel,
+                    code,
+                    status_class,
+                } => {
                     // Cross-model threshold: 3 distinct ids for the
                     // channel — or all configured ids when fewer than
                     // three are configured. Below the threshold the
                     // finding is recorded without action.
+                    //
+                    // evallerina-xc3 (conservative reading): distinct
+                    // models are counted WITHIN one status class — the
+                    // status_class key dimension splits the tallies, so
+                    // an invalid_output occurrence can never push a
+                    // failed-class hint-blindness finding over the
+                    // threshold.
+                    //
+                    // Derivation rule (AC-2): hint-blindness findings are
+                    // the status_class="failed" entries; the
+                    // status_class="invalid_output" entries carry
+                    // ERR_ACTION_FORMAT_VIOLATION from empty-replay
+                    // trials and are never folded into those counts.
                     let kind = if distinct_models >= routing_threshold {
                         "aix-channel"
                     } else {
@@ -353,6 +396,7 @@ pub fn aggregate_rows(
                         "kind": kind,
                         "channel": channel,
                         "code": code,
+                        "status_class": status_class,
                         "distinct_models": distinct_models,
                         "occurrences": tally.occurrences,
                         "scenarios": tally.scenarios.iter().collect::<Vec<_>>(),
@@ -395,7 +439,7 @@ pub fn aggregate_rows(
 /// row carries model attribution, into the tier-2 matrix cell.
 fn tally(
     routings: &mut BTreeMap<FaultKey, FaultTally>,
-    matrix_cells: &mut BTreeMap<(String, String, String), FaultTally>,
+    matrix_cells: &mut BTreeMap<(String, String, String, String), FaultTally>,
     key: &FaultKey,
     scenario: &str,
     model: Option<&str>,
@@ -406,11 +450,14 @@ fn tally(
     entry.scenarios.insert(scenario.to_owned());
     if let Some(model) = model {
         entry.models.insert(model.to_owned());
-        let code = match key {
-            FaultKey::Agent { code, .. } => code.clone(),
+        let (code, status_class) = match key {
+            FaultKey::Agent {
+                code, status_class, ..
+            } => (code.clone(), status_class.clone()),
             // Tool faults carry no code; the matrix dimension uses the
-            // check name so tool rows stay visible per capability.
-            FaultKey::Tool { .. } => check.to_owned(),
+            // check name so tool rows stay visible per capability, and
+            // no status_class (empty string keeps the prior cell shape).
+            FaultKey::Tool { .. } => (check.to_owned(), String::new()),
         };
         let cell = matrix_cells
             .entry((
@@ -421,6 +468,7 @@ fn tally(
                 },
                 code,
                 model.to_owned(),
+                status_class,
             ))
             .or_default();
         cell.occurrences += 1;
diff --git a/tests/report_aggregation.rs b/tests/report_aggregation.rs
index b58d798..cb172c1 100644
--- a/tests/report_aggregation.rs
+++ b/tests/report_aggregation.rs
@@ -169,6 +169,58 @@ fn invalid_output_lands_in_matrix_with_action_format_violation() {
     assert_eq!(cells[0]["occurrences"], 1);
 }
 
+/// EDGE-003 (evallerina-xc3): a failed trial's hint-blindness fault and a
+/// trial-level `invalid_output` fault (ERR_ACTION_FORMAT_VIOLATION) in the
+/// SAME channel are two distinct matrix AND routing entries with distinct
+/// `status_class` values — the invalid_output occurrence is never blended
+/// into the failed-trial hint-blindness counts (its occurrence count
+/// excludes it).
+#[test]
+fn invalid_output_never_blends_into_failed_hint_blindness() {
+    let rows = vec![
+        trial(
+            "wai-hint-adherence-typo",
+            "m1",
+            "failed",
+            json!([agent_fault("ERR_DOC_DRIFT_BLINDNESS")]),
+        ),
+        trial("wai-hint-adherence-typo", "m1", "invalid_output", json!([])),
+    ];
+    let dashboard = aggregate_rows(&rows, &models(3), &test_channel_of).expect("aggregates");
+
+    // Matrix: two distinct entries, distinct status_class values.
+    let cells = dashboard["matrix"].as_array().unwrap();
+    assert_eq!(cells.len(), 2);
+    let hint = cells
+        .iter()
+        .find(|c| c["code"] == "ERR_DOC_DRIFT_BLINDNESS")
+        .expect("hint-blindness cell present");
+    assert_eq!(hint["status_class"], "failed");
+    assert_eq!(hint["occurrences"], 1);
+    let invalid = cells
+        .iter()
+        .find(|c| c["code"] == "ERR_ACTION_FORMAT_VIOLATION")
+        .expect("invalid_output cell present");
+    assert_eq!(invalid["status_class"], "invalid_output");
+    assert_eq!(invalid["occurrences"], 1);
+
+    // Routing: two distinct agent-fault entries, same separation.
+    let routings = dashboard["routing"].as_array().unwrap();
+    assert_eq!(routings.len(), 2);
+    let hint_route = routings
+        .iter()
+        .find(|r| r["code"] == "ERR_DOC_DRIFT_BLINDNESS")
+        .expect("hint-blindness routing entry present");
+    assert_eq!(hint_route["status_class"], "failed");
+    assert_eq!(hint_route["occurrences"], 1);
+    let invalid_route = routings
+        .iter()
+        .find(|r| r["code"] == "ERR_ACTION_FORMAT_VIOLATION")
+        .expect("invalid_output routing entry present");
+    assert_eq!(invalid_route["status_class"], "invalid_output");
+    assert_eq!(invalid_route["occurrences"], 1);
+}
+
 // ── fault routing (evals-guidelines) ────────────────────────────────────────
 
 #[test]
```

### Test results

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running tests/report_aggregation.rs (target/debug/deps/report_aggregation-96a95a77eea827f0)

running 25 tests
test absent_rows_never_routed ... ok
test absent_cells_surfaced_never_failed ... ok
test dashboard_has_no_single_aggregate_score ... ok
test archetype_scenarios_declare_channels ... ok
test file_without_route_is_an_error ... ok
test http_error_rows_surfaced_without_fault ... ok
test invalid_output_lands_in_matrix_with_action_format_violation ... ok
test empty_configured_registry_is_an_error ... ok
test matrix_cell_accumulates_occurrences ... ok
test invalid_output_never_blends_into_failed_hint_blindness ... ok
test matrix_groups_agent_faults_by_channel_code_model ... ok
test rate_limited_rows_surfaced_without_fault ... ok
test passing_checks_count_in_coverage ... ok
test rows_without_model_attribution_group_as_non_tier2 ... ok
test route_dry_run_attaches_routed_summary_additively ... ok
test same_code_across_different_channels_stays_split ... ok
test small_registry_routes_when_all_configured_ids_hit ... ok
test same_code_across_threshold_models_routes_to_aix ... ok
test unknown_scenario_channel_is_an_error ... ok
test sub_threshold_code_recorded_without_action ... ok
test tool_fault_routes_to_tool_ticket_never_aix ... ok
test unmapped_tool_counts_skipped_and_report_succeeds ... ok
test report_without_route_has_no_routed_field_and_no_jsonl ... ok
test report_route_dry_run_writes_proposed_tickets_jsonl ... ok
test every_live_scenario_declares_a_channel ... ok


test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### Fresh report output

```json
{
  "configured_models": [
    "openrouter/free",
    "cohere/north-mini-code:free",
    "google/gemma-4-26b-a4b-it:free",
    "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free"
  ],
  "coverage": {
    "absent": 0,
    "checks_failed": 61,
    "checks_passed": 16,
    "failed": 13,
    "http_error": 56,
    "invalid_output": 3,
    "non_tier2": 0,
    "passed": 0,
    "rate_limited": 0,
    "trials": 72
  },
  "dashboard_version": 1,
  "matrix": [
    {
      "channel": "artifact",
      "code": "ERR_ACTION_FORMAT_VIOLATION",
      "model": "openrouter/free",
      "occurrences": 1,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "artifact",
      "code": "ERR_DOC_DRIFT_BLINDNESS",
      "model": "openrouter/free",
      "occurrences": 1,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "failed"
    },
    {
      "channel": "artifact",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "model": "openrouter/free",
      "occurrences": 4,
      "scenarios": [
        "dont-doc-drift-stale-lifecycle",
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "failed"
    },
    {
      "channel": "artifact",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "model": "openrouter/free",
      "occurrences": 1,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "artifact",
      "code": "agent-trusted-envelope-not-stale-docs",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "dont-doc-drift-stale-lifecycle",
        "wai-doc-drift-stale-agents-md"
      ]
    },
    {
      "channel": "artifact",
      "code": "init-recovery-ok",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ]
    },
    {
      "channel": "artifact",
      "code": "show-reports-verified",
      "model": "openrouter/free",
      "occurrences": 1,
      "scenarios": [
        "dont-doc-drift-stale-lifecycle"
      ]
    },
    {
      "channel": "envelope",
      "code": "ERR_ACTION_FORMAT_VIOLATION",
      "model": "openrouter/free",
      "occurrences": 2,
      "scenarios": [
        "wai-hint-adherence-corrupt-config",
        "wai-hint-adherence-smoke"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "envelope",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "model": "openrouter/free",
      "occurrences": 13,
      "scenarios": [
        "dont-hint-adherence-lifecycle",
        "wai-hint-adherence-corrupt-config",
        "wai-hint-adherence-smoke"
      ],
      "status_class": "failed"
    },
    {
      "channel": "envelope",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "model": "openrouter/free",
      "occurrences": 2,
      "scenarios": [
        "wai-hint-adherence-corrupt-config",
        "wai-hint-adherence-smoke"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "envelope",
      "code": "config-error-envelope-carries-remediation-hint",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-corrupt-config"
      ]
    },
    {
      "channel": "envelope",
      "code": "file-uri-rejected-with-repair-hint",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ]
    },
    {
      "channel": "envelope",
      "code": "flag-error-carries-evidence-remediation",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ]
    },
    {
      "channel": "envelope",
      "code": "init-recovery-ok",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-smoke"
      ]
    },
    {
      "channel": "envelope",
      "code": "show-reports-verified",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ]
    },
    {
      "channel": "envelope",
      "code": "status-error-envelope-carries-remediation-hint",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-smoke"
      ]
    },
    {
      "channel": "envelope",
      "code": "status-recovery-ok",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-corrupt-config"
      ]
    },
    {
      "channel": "envelope",
      "code": "trust-error-carries-reason-remediation",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ]
    },
    {
      "channel": "suggestions",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-typo"
      ],
      "status_class": "failed"
    },
    {
      "channel": "suggestions",
      "code": "init-recovery-ok",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-typo"
      ]
    },
    {
      "channel": "suggestions",
      "code": "typo-stderr-carries-did-you-mean",
      "model": "openrouter/free",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-typo"
      ]
    }
  ],
  "report_version": 1,
  "routing": [
    {
      "channel": "artifact",
      "code": "ERR_ACTION_FORMAT_VIOLATION",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 1,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "artifact",
      "code": "ERR_DOC_DRIFT_BLINDNESS",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 1,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "failed"
    },
    {
      "channel": "artifact",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 4,
      "scenarios": [
        "dont-doc-drift-stale-lifecycle",
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "failed"
    },
    {
      "channel": "artifact",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 1,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "envelope",
      "code": "ERR_ACTION_FORMAT_VIOLATION",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 2,
      "scenarios": [
        "wai-hint-adherence-corrupt-config",
        "wai-hint-adherence-smoke"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "envelope",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 13,
      "scenarios": [
        "dont-hint-adherence-lifecycle",
        "wai-hint-adherence-corrupt-config",
        "wai-hint-adherence-smoke"
      ],
      "status_class": "failed"
    },
    {
      "channel": "envelope",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 2,
      "scenarios": [
        "wai-hint-adherence-corrupt-config",
        "wai-hint-adherence-smoke"
      ],
      "status_class": "invalid_output"
    },
    {
      "channel": "suggestions",
      "code": "ERR_ENVELOPE_HINT_BLINDNESS",
      "distinct_models": 1,
      "kind": "recorded-no-action",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-typo"
      ],
      "status_class": "failed"
    },
    {
      "channel": "artifact",
      "check": "agent-trusted-envelope-not-stale-docs",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "dont-doc-drift-stale-lifecycle",
        "wai-doc-drift-stale-agents-md"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "config-error-envelope-carries-remediation-hint",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-corrupt-config"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "file-uri-rejected-with-repair-hint",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "flag-error-carries-evidence-remediation",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ],
      "tool": "wai"
    },
    {
      "channel": "artifact",
      "check": "init-recovery-ok",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-doc-drift-stale-agents-md"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "init-recovery-ok",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-smoke"
      ],
      "tool": "wai"
    },
    {
      "channel": "suggestions",
      "check": "init-recovery-ok",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-typo"
      ],
      "tool": "wai"
    },
    {
      "channel": "artifact",
      "check": "show-reports-verified",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 1,
      "scenarios": [
        "dont-doc-drift-stale-lifecycle"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "show-reports-verified",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "status-error-envelope-carries-remediation-hint",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-smoke"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "status-recovery-ok",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-corrupt-config"
      ],
      "tool": "wai"
    },
    {
      "channel": "envelope",
      "check": "trust-error-carries-reason-remediation",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "dont-hint-adherence-lifecycle"
      ],
      "tool": "wai"
    },
    {
      "channel": "suggestions",
      "check": "typo-stderr-carries-did-you-mean",
      "distinct_models": 1,
      "kind": "tool-ticket",
      "occurrences": 3,
      "scenarios": [
        "wai-hint-adherence-typo"
      ],
      "tool": "wai"
    }
  ],
  "routing_threshold": 3
}
```

### Issue status

```text
◐ evallerina-xc3 · Report aggregation: report ERR_ACTION_FORMAT_VIOLATION separately from hint-blindness counts   [P3 · IN_PROGRESS]
Created by: charly vibes · Assignee: charly vibes · Type: task
Created: 2026-10-26 · Started: 2026-10-26 · Updated: 2026-10-26
Lease: expires expired (heartbeat 5 mins ago)

description

  Review finding EDGE-003 (mechanically verified from Oct 12/19 artifacts).
  Three invalid_output rows per run (empty replay, fault_code                  
  ERR_ACTION_FORMAT_VIOLATION) were folded into the hint-blindness narrative   
  in the investigation summary. Those agents produced no valid action at all —
  a format violation, qualitatively different from exploratory hint-blindness  
  (rich replays showing ls/find/stat wandering). Blending the two overstates   
  the envelope-hint signal and corrupts the fault taxonomy that routes agent   
  faults to genesis AIX work.                                                  


ACCEPTANCE CRITERIA

                                                                               
  1. evallerina report / dashboard groups fault codes by trial status class    
  (failed vs invalid_output vs http_error). 2. Investigation-style summaries   
  derive hint-blindness counts only from trials with non-empty replays. 3.     
  Test covering the separation.                                                


LABELS: afk, complexity:s

METADATA
  base_commit: 455223b6a7e8f2f14c8af49f42a0c23d4b3c2859
  files: ["src/report.rs","tests/report_aggregation.rs"]
```

## Verdict

VERDICT: CONFIRM — implementation matches the claims. Reads cited:
brief .wai/projects/evallerina/briefs/evallerina-xc3.md and this artifact
.wai/projects/evallerina/research/verify-evallerina-xc3.md. Fresh test-run
output pasted above (test result: ok. 25 passed; 0 failed) plus the fresh
dashboard run on reports/tier2-rotation-2026-10-19.jsonl showing
ERR_ACTION_FORMAT_VIOLATION keyed status_class="invalid_output" strictly
separate from status_class="failed" hint-blindness entries. All evidence artifacts are captured in `.wai/projects/evallerina/research/verify-evallerina-xc3.md`.