//! Purpose: Contract tests for the A/B ablation harness (evallerina-rl1,
//! evals.md Step 5).
//! Responsibilities: pin the A/B shape — full arm (llms.txt + managed
//! AGENTS.md + .genesis/tools.toml provisioned into the fixture) vs
//! ablated arm (raw binaries, no AIX context), delta computed over ≥3
//! runs per arm, absent and rate-limited cells surfaced without being
//! counted as failures, arm recorded additively on tier-2 rows
//! (report_version 1 unchanged), and loud refusal on artifact-path
//! collisions instead of silently overwriting scenario fixtures.
//! Rationale: the score delta is the measured value of the AIX
//! investment; a harness that fakes a delta (undefined → 0, dropped
//! cells, sub-3 samples) would manufacture value the evals never
//! measured.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use genesis::evals::{CheckOutcome, Scenario};
use genesis::managed_block::BlockDef;
use serde_json::json;

use evallerina::ablation::{
    AblationFactories, AixArtifacts, AixSource, ToolBinaries, run_ablation,
};
use evallerina::live::{Arm, Bounds, RotationEvent, Transport, TransportError, TrialStatus};

/// A fresh unique temp dir for real-tool generation fixtures. Deleted
/// after use by the tests that create it.
fn unique_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "evallerina-ablation-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
    ));
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

/// The dulce-family AIX artifact set (test content; production wires
/// the real generated artifacts).
fn test_artifacts() -> AixArtifacts {
    AixArtifacts {
        llms_txt: "llms.txt content: wai status --json",
        managed_agents_md: "<!-- WAI:START -->managed guidance<!-- WAI:END -->",
        tools_toml: "[tools]\nwai = { cmd = \"wai\" }\n",
    }
}

/// Probe scenario: the check passes iff the artifact `llms.txt` is
/// provisioned in the fixture — so the full arm passes and the ablated
/// arm fails, making the delta fully scripted and deterministic.
fn artifacts_scenario() -> Scenario {
    Scenario::new("ablation-probe", "create the file").check("artifacts-present", |r| {
        if r.fixture_root.join("llms.txt").exists() {
            CheckOutcome::pass()
        } else {
            CheckOutcome::tool_fault("llms.txt missing from fixture")
        }
    })
}

/// A transport with a scripted completion list.
struct FakeTransport {
    completions: Vec<Result<String, TransportError>>,
}

impl FakeTransport {
    fn new(completions: Vec<Result<String, TransportError>>) -> Self {
        Self { completions }
    }
}

impl Transport for FakeTransport {
    fn complete(
        &mut self,
        _messages: &[evallerina::live::Message],
    ) -> Result<String, TransportError> {
        if self.completions.is_empty() {
            return Err(TransportError::Http("script exhausted".into()));
        }
        self.completions.remove(0)
    }
}

fn action(command: &str, done: bool) -> String {
    json!({ "command": command, "done": done }).to_string()
}

/// A green two-turn script: run a command, then declare done.
fn green_turns() -> Vec<Result<String, TransportError>> {
    vec![
        Ok(action("printf ok > touched.txt", false)),
        Ok(action("true", true)),
    ]
}

/// A green script sized for `trials` two-turn trials (the factory is
/// called once per model, and one transport serves the whole cell).
fn green_script(trials: u32) -> Vec<Result<String, TransportError>> {
    let mut completions = Vec::new();
    for _ in 0..trials {
        completions.extend(green_turns());
    }
    completions
}

/// Factory whose transports complete every task green.
fn green_factory() -> impl FnMut(&str) -> Result<Box<dyn Transport>, String> {
    move |_model| Ok(Box::new(FakeTransport::new(green_script(3))) as Box<dyn Transport>)
}

/// Factory whose transports are always rate-limited (429, retry also
/// 429 → each trial ends `rate_limited`, no fault attribution).
fn rate_limited_factory() -> impl FnMut(&str) -> Result<Box<dyn Transport>, String> {
    move |_model| {
        Ok(Box::new(FakeTransport::new(vec![
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
            Err(TransportError::RateLimited),
        ])) as Box<dyn Transport>)
    }
}

fn always_open() -> bool {
    true
}

/// A shared budget that stays open for `n` trial starts, then closes
/// (deterministic mid-ablation expiry without sleeping).
fn budget_closes_after(n: u32) -> impl Fn() -> bool {
    let remaining = std::cell::Cell::new(n);
    move || {
        if remaining.get() == 0 {
            return false;
        }
        remaining.set(remaining.get() - 1);
        true
    }
}

// ── real artifact generation (opt-in; evallerina-0j8) ───────────────────────

/// Real generation: `wai init --json` inside the generation fixture
/// yields the real managed AGENTS.md — wai's provenance footer and the
/// managed block markers from genesis-vibes, never placeholder text.
/// Skips hermetically when no `wai` binary is available (CI without
/// tool binaries); runs for real wherever the tool is provisioned.
#[test]
fn real_generation_writes_managed_agents_md_with_wai_provenance() {
    let Some(wai) = ToolBinaries::discover_wai() else {
        return; // no tool binaries provisioned → hermetic skip
    };
    let dir = unique_dir("real-agents-md");
    let generated =
        AixArtifacts::for_fixture(&dir, &ToolBinaries { wai }).expect("wai init generates");
    let block = BlockDef::new("WAI");
    assert!(
        generated.managed_agents_md.contains(&block.start_marker),
        "real AGENTS.md carries the managed block start marker"
    );
    assert!(
        generated.managed_agents_md.contains(&block.end_marker),
        "real AGENTS.md carries the managed block end marker"
    );
    assert!(
        generated
            .managed_agents_md
            .contains("provenance: generator=wai"),
        "real AGENTS.md carries wai's provenance footer"
    );
    assert!(
        !generated.managed_agents_md.contains("managed guidance"),
        "real AGENTS.md is never the placeholder text"
    );
    // wai init also registers the tool manifest via genesis discovery:
    // the produced `.genesis/tools.toml` is the real registered one.
    assert!(
        generated.tools_toml.contains("[tools.wai]"),
        "real tools.toml registers wai: {}",
        generated.tools_toml
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Real generation: the fixture's `llms.txt` is the genesis-vibes
/// `generate_llms_txt` output for the fixture tool set — title, module
/// section markers, and the registered tool entries, never placeholder
/// text. Runs for real wherever the tool is provisioned.
#[test]
fn real_generation_llms_txt_is_genesis_output_for_the_tool_set() {
    let Some(wai) = ToolBinaries::discover_wai() else {
        return; // no tool binaries provisioned → hermetic skip
    };
    let dir = unique_dir("real-llms-txt");
    let generated =
        AixArtifacts::for_fixture(&dir, &ToolBinaries { wai }).expect("wai init generates");
    // The standardized concise format: `# {project}` title and the
    // module listing of the registered fixture tool set.
    assert!(
        generated.llms_txt.starts_with("# "),
        "llms.txt carries the project title: {}",
        generated.llms_txt
    );
    assert!(
        generated.llms_txt.contains("## Modules"),
        "llms.txt carries the module section marker: {}",
        generated.llms_txt
    );
    assert!(
        generated.llms_txt.contains("`wai`"),
        "llms.txt lists the registered tool: {}",
        generated.llms_txt
    );
    assert!(
        !generated.llms_txt.contains("wai status --json"),
        "llms.txt is never the placeholder text"
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ── hermetic fallback (opt-out / no tool binaries) ─────────────────────────

/// Without the real-generation opt-in (no tool binaries provisioned,
/// or the caller simply doesn't opt in) the harness stays hermetic:
/// the full arm provisions exactly today's static contents — behavior
/// byte-for-byte unchanged.
#[test]
fn static_source_provisions_hermetic_contents_unchanged() {
    let static_md = test_artifacts().managed_agents_md;
    let scenario =
        Scenario::new("hermetic-probe", "create the file").check("agents-md-is-static", move |r| {
            let md = std::fs::read_to_string(r.fixture_root.join("AGENTS.md")).unwrap_or_default();
            if md == static_md {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("AGENTS.md is not the hermetic static content")
            }
        });
    let report = run_ablation(
        &scenario,
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 3, "full arm sees the static contents");
    assert_eq!(report.ablated.failed, 3, "ablated arm sees none of them");
}

// ── real generation failures are loud (evallerina-0j8 step 4) ───────────────

/// A fake tool binary: a shell script with the given body, executable.
fn fake_wai(tag: &str, body: &str) -> std::path::PathBuf {
    let dir = unique_dir(tag);
    let script = dir.join("wai");
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).expect("write fake wai");
    #[allow(clippy::permissions_set_readonly_false)] // executable bit
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("chmod fake wai");
    script
}

/// A `wai` binary that fails (nonzero exit) makes real generation a
/// loud error — and `run_ablation` aborts with that error rather than
/// silently provisioning placeholder contents.
#[test]
fn tool_generation_failure_is_a_loud_error() {
    let failing = fake_wai("failing-wai", "echo boom >&2\nexit 3");
    let dir = unique_dir("failing-wai-fixture");
    let err = AixArtifacts::for_fixture(
        &dir,
        &ToolBinaries {
            wai: failing.clone(),
        },
    )
    .expect_err("generation failure is loud");
    assert!(
        err.contains("wai init"),
        "error names the failed tool: {err}"
    );
    // run_ablation propagates the loud error: no silent placeholder
    // fallback, no ablation with fake artifacts.
    let err = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        AixSource::Real(ToolBinaries { wai: failing }),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect_err("ablation aborts on generation failure");
    assert!(
        err.contains("wai init"),
        "error names the failed tool: {err}"
    );
}

/// A `wai` binary that exits 0 but produces nothing (no AGENTS.md) is
/// also a loud error — the produced files are asserted, not assumed.
#[test]
fn missing_produced_file_is_a_loud_error() {
    let silent = fake_wai("silent-wai", "exit 0");
    let dir = unique_dir("silent-wai-fixture");
    let err = AixArtifacts::for_fixture(&dir, &ToolBinaries { wai: silent })
        .expect_err("missing artifact is loud");
    assert!(
        err.contains("AGENTS.md"),
        "error names the missing produced file: {err}"
    );
}

/// The corrupt-config concern: `.genesis/tools.toml` can be
/// scenario-owned (the fault injection). Real mode refuses the same
/// collision the static mode refuses — before any generation runs, so
/// a tool never writes into scenario material.
#[test]
fn real_mode_refuses_scenario_owned_paths_before_generation() {
    let failing = fake_wai("collision-wai", "exit 0");
    let scenario = Scenario::new("colliding-real", "create the file")
        .fixture_file(".genesis/tools.toml", "scenario-owned corrupt config")
        .check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    let err = run_ablation(
        &scenario,
        &["m1".to_owned()],
        3,
        AixSource::Real(ToolBinaries { wai: failing }),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect_err("collision refused in real mode too");
    assert!(
        err.contains("tools.toml"),
        "error names the colliding path: {err}"
    );
}

// ── the A/B shape ───────────────────────────────────────────────────────────

/// The Must gate: an A/B report for one scenario with the delta
/// computed over ≥3 runs per arm. The full arm passes because the
/// artifacts are provisioned; the ablated arm fails the same check —
/// the delta is entirely the artifact effect.
#[test]
fn delta_computed_over_three_runs_per_arm() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 3);
    assert_eq!(report.ablated.failed, 3);
    assert_eq!(report.delta, Some(1.0));
}

/// The full arm sees the three AIX artifacts in its fixture; the
/// ablated arm sees none (raw binaries only).
#[test]
fn artifact_fixture_files_map_the_three_paths() {
    let files = test_artifacts().fixture_files();
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths, ["AGENTS.md", ".genesis/tools.toml", "llms.txt"]);
}

// ── anti-goals: no manufactured deltas ──────────────────────────────────────

/// The Must gate floor is three runs per arm; fewer samples cannot
/// support a delta claim. Refuse loudly, never run a thin A/B.
#[test]
fn reps_below_three_is_an_error() {
    let err = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        2,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect_err("reps < 3 refused");
    assert!(err.contains("3"), "error names the floor: {err}");
}

/// An arm where nothing ran has no pass rate: the delta is undefined,
/// recorded as None — never silently zero, which would read as "no
/// measured difference".
#[test]
fn delta_undefined_when_an_arm_cannot_run() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut rate_limited_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.ablated.trials, 3);
    assert_eq!(report.ablated.rate_limited, 3);
    assert_eq!(report.ablated.pass_rate, None);
    assert_eq!(report.delta, None);
}

/// Budget-skipped cells are recorded absent, never failed, and are
/// excluded from the pass-rate denominator while still being surfaced.
#[test]
fn absent_cells_surfaced_and_excluded_from_delta_denominator() {
    // 3 budget ticks for the full arm, 1 for the ablated arm; the
    // remaining 2 ablated cells never start → absent.
    let budget = budget_closes_after(4);
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &budget,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 3);
    assert_eq!(report.ablated.trials, 1);
    assert_eq!(report.ablated.failed, 1);
    assert_eq!(report.ablated.absent, 2);
    assert_eq!(report.delta, Some(1.0));
}

// ── arm attribution on tier-2 rows (additive, version unchanged) ────────────

/// Tier-2 rows record the arm so concatenated rotation + ablation
/// output stays attributable; the field is additive, report_version
/// stays 1 (evals-guidelines additive evolution).
#[test]
fn rows_carry_arm_and_report_version_stays_1() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    for event in &report.full_rows {
        match event {
            RotationEvent::Trial { report } => {
                assert_eq!(report.arm, Arm::Full);
                assert_eq!(report.report_version, 1);
            }
            RotationEvent::Absent { arm, .. } => assert_eq!(*arm, Arm::Full),
        }
    }
    for event in &report.ablated_rows {
        match event {
            RotationEvent::Trial { report } => assert_eq!(report.arm, Arm::Ablated),
            RotationEvent::Absent { arm, .. } => assert_eq!(*arm, Arm::Ablated),
        }
    }
}

// ── fixture integrity ───────────────────────────────────────────────────────

/// Provisioning never silently overwrites scenario material: an
/// artifact path colliding with a scenario fixture is a loud error —
/// the agent must not observe an artifact where the scenario injected
/// a fault (e.g. a corrupt config).
#[test]
fn artifact_path_collision_is_a_loud_error() {
    let scenario = Scenario::new("colliding", "create the file")
        .fixture_file("AGENTS.md", "scenario-owned content")
        .check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    let err = run_ablation(
        &scenario,
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect_err("collision refused");
    assert!(
        err.contains("AGENTS.md"),
        "error names the colliding path: {err}"
    );
}

// ── report shape ────────────────────────────────────────────────────────────

/// The A/B report serializes with per-arm detail and its own version;
/// the only score is the per-scenario delta — no single aggregate
/// number across scenarios exists.
#[test]
fn ablation_report_serializes_with_per_arm_detail() {
    let report = run_ablation(
        &artifacts_scenario(),
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut green_factory(),
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    let rendered = serde_json::to_value(&report).expect("serializes");
    assert_eq!(rendered["ablation_version"], 1);
    assert_eq!(rendered["scenario"], "ablation-probe");
    assert_eq!(rendered["full"]["arm"], "full");
    assert_eq!(rendered["ablated"]["arm"], "ablated");
    assert_eq!(rendered["full"]["pass_rate"], 1.0);
    assert_eq!(rendered["delta"], 1.0);
    assert!(rendered.get("score").is_none());
    assert!(rendered.get("aggregate").is_none());
    // The report is auditable: it carries the underlying rows verbatim.
    assert_eq!(rendered["full_rows"].as_array().map(Vec::len), Some(3));
    assert_eq!(rendered["ablated_rows"].as_array().map(Vec::len), Some(3));
}

/// A mixed delta stays exact: full 2/3 vs ablated 0/3 → 2/3. This
/// probe's check depends on the agent's action (touched.txt), so a
/// scripted failure inside the full arm is observable.
#[test]
fn mixed_results_produce_fractional_delta() {
    let action_scenario =
        Scenario::new("ablation-probe-action", "create the file").check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    // Script trial 2's turns to declare done without ever writing the
    // file; trials 1 and 3 stay green. Turn layout: trial n owns
    // completions [2n, 2n+1] — overwrite both of trial 2's turns.
    let mut full_factory = move |_model: &str| {
        let mut scripted = green_script(3);
        scripted[2] = Ok(action("true", false));
        scripted[3] = Ok(action("true", true));
        Ok(Box::new(FakeTransport::new(scripted)) as Box<dyn Transport>)
    };
    let report = run_ablation(
        &action_scenario,
        &["m1".to_owned()],
        3,
        AixSource::Static(&test_artifacts()),
        AblationFactories {
            full: &mut full_factory,
            ablated: &mut green_factory(),
        },
        &Bounds::default(),
        &always_open,
    )
    .expect("ablation runs");
    assert_eq!(report.full.passed, 2);
    assert_eq!(report.full.failed, 1);
    assert_eq!(report.ablated.passed, 3);
    let delta = report.delta.expect("both arms ran");
    assert!((delta - (-1.0 / 3.0)).abs() < 1e-9);
}

/// `Arm` serializes snake_case and the plain (non-ablation) rotation
/// rows attribute to the full arm by default.
#[test]
fn plain_rotation_rows_default_to_full_arm() {
    use evallerina::live::{TransportFactory, run_rotation};
    let action_scenario =
        Scenario::new("ablation-probe-action", "create the file").check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("touched.txt missing")
            }
        });
    let mut factory: TransportFactory =
        &mut |_model| Ok(Box::new(FakeTransport::new(green_script(1))) as Box<dyn Transport>);
    let events = run_rotation(
        &[action_scenario],
        &["m1".to_owned()],
        1,
        &mut factory,
        &Bounds::default(),
        &always_open,
    )
    .expect("rotation runs");
    match &events[0] {
        RotationEvent::Trial { report } => {
            assert_eq!(report.arm, Arm::Full);
            assert!(matches!(report.status, TrialStatus::Passed));
        }
        RotationEvent::Absent { .. } => panic!("open budget never skips"),
    }
}
