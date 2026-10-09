//! Purpose: Tier-2 live-runner protocol tests, fully offline.
//! Responsibilities: drive `evallerina::live::run_trial` with a scripted
//! fake transport and assert the evals-guidelines Live action protocol:
//! one structured action per turn, one re-ask on malformed output,
//! bounds with recorded values, 429 retry-once semantics, and the
//! tier-2 report row shape (verbatim model id, repetition index).
//! Rationale: the protocol is the contract; the real OpenRouter client
//! is a thin adapter behind the same `Transport` seam and requires
//! OPENROUTER_API_KEY at runtime, never in tests.

use evallerina::live::{
    Bounds, Message, RotationEvent, Transport, TransportError, TrialStatus, run_rotation, run_trial,
};
use genesis::evals::Scenario;

/// A transport that replays scripted completions in order.
struct FakeTransport {
    completions: Vec<Result<String, TransportError>>,
    /// Captured per-turn message traffic for budget/protocol assertions.
    seen_turns: Vec<Vec<Message>>,
}

impl FakeTransport {
    fn new(completions: Vec<Result<String, TransportError>>) -> Self {
        Self {
            completions,
            seen_turns: Vec::new(),
        }
    }
}

impl Transport for FakeTransport {
    fn complete(&mut self, messages: &[Message]) -> Result<String, TransportError> {
        self.seen_turns.push(messages.to_vec());
        if self.completions.is_empty() {
            return Err(TransportError::Http("script exhausted".into()));
        }
        self.completions.remove(0)
    }
}

/// A well-formed action: run a command, then stop.
fn action(command: &str, done: bool) -> String {
    serde_json::json!({ "command": command, "done": done }).to_string()
}

/// Scenario with one check that passes when the fixture file exists.
fn probe_scenario() -> Scenario {
    Scenario::new("probe", "create the file")
        .fixture_file("AGENTS.md", "fixture body")
        .check("touch-landed", |r| {
            if r.fixture_root.join("touched.txt").exists() {
                genesis::evals::CheckOutcome::pass()
            } else {
                genesis::evals::CheckOutcome::tool_fault("touched.txt missing")
            }
        })
}

fn default_bounds() -> Bounds {
    Bounds::default()
}

// ── rotation (evallerina-aay) ───────────────────────────────────────────────

/// Scripted fake-transport factory for rotation tests: one fresh transport
/// per model, each with enough well-formed completions for the reps asked.
fn fake_factory(
    mut make_completion: impl FnMut() -> Result<String, TransportError>,
) -> impl FnMut(&str) -> Result<Box<dyn Transport>, String> {
    move |_model| {
        // Standard green trial: touch the fixture, then declare done.
        Ok(Box::new(FakeTransport::new(vec![
            Ok(action("printf ok > touched.txt", false)),
            make_completion(),
        ])) as Box<dyn Transport>)
    }
}

/// An always-true budget: every cell runs.
fn budget_open() -> bool {
    true
}

/// A budget that stays open for `n` calls, then closes (deterministic
/// mid-rotation expiry without sleeping).
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

/// With an open budget the whole registry runs in order, one trial event
/// per cell, and every event is a trial (never absent, never failed).
#[test]
fn rotation_runs_whole_registry_in_order_when_budget_open() {
    let mut factory = fake_factory(|| Ok(action("true", true)));
    let models: Vec<String> = evallerina::registry::ordered_ids()
        .into_iter()
        .map(String::from)
        .collect();
    let scenarios = [probe_scenario()];
    let events = run_rotation(
        &scenarios,
        &models,
        1,
        &mut factory,
        &default_bounds(),
        &budget_open,
    )
    .expect("rotation runs");

    assert_eq!(events.len(), models.len());
    for (event, model) in events.iter().zip(&models) {
        match event {
            RotationEvent::Trial { report } => {
                assert_eq!(report.status, TrialStatus::Passed);
                assert_eq!(&report.model, model);
                assert_eq!(report.report_version, 1);
            }
            RotationEvent::Absent { .. } => {
                panic!("no cell may be absent when the budget is open")
            }
        }
    }
}

/// An exhausted budget records every unstarted cell as `absent` — never
/// as a failed row and never silently dropped.
#[test]
fn rotation_records_unstarted_cells_absent_never_failed() {
    let mut factory = fake_factory(|| Ok(action("true", true)));
    let models: Vec<String> = evallerina::registry::ordered_ids()
        .into_iter()
        .map(String::from)
        .collect();
    let scenarios = [probe_scenario()];
    let events = run_rotation(
        &scenarios,
        &models,
        1,
        &mut factory,
        &default_bounds(),
        &budget_closes_after(0),
    )
    .expect("rotation runs");

    assert_eq!(events.len(), models.len());
    for (event, model) in events.iter().zip(&models) {
        match event {
            RotationEvent::Trial { .. } => {
                panic!("no trial may run past an exhausted budget")
            }
            RotationEvent::Absent {
                scenario,
                model: absent_model,
                repetition,
                ..
            } => {
                assert_eq!(scenario, "probe");
                assert_eq!(absent_model, model);
                assert_eq!(*repetition, 0);
            }
        }
    }
}

/// Budget expiry mid-rotation: the trial already started runs to
/// completion, the remaining repetitions of the rotation are recorded
/// absent at trial granularity.
#[test]
fn rotation_partial_budget_finishes_started_trial_rest_absent() {
    let mut factory = fake_factory(|| Ok(action("true", true)));
    let models: Vec<String> = evallerina::registry::ordered_ids()
        .into_iter()
        .map(String::from)
        .collect();
    let scenarios = [probe_scenario()];
    let events = run_rotation(
        &scenarios,
        &models,
        1,
        &mut factory,
        &default_bounds(),
        &budget_closes_after(1),
    )
    .expect("rotation runs");

    assert_eq!(events.len(), models.len());
    assert!(matches!(events[0], RotationEvent::Trial { .. }));
    assert!(
        events[1..]
            .iter()
            .all(|e| matches!(e, RotationEvent::Absent { .. }))
    );
}

/// A transport-factory failure (e.g. missing API key) aborts the rotation
/// with an error — silent degradation is the anti-goal.
#[test]
fn rotation_factory_error_aborts_loudly() {
    let models: Vec<String> = vec!["m1".into(), "m2".into()];
    let mut factory = |_model: &str| -> Result<Box<dyn Transport>, String> {
        Err("OPENROUTER_API_KEY is not set".into())
    };
    let scenarios = [probe_scenario()];
    let err = run_rotation(
        &scenarios,
        &models,
        1,
        &mut factory,
        &default_bounds(),
        &budget_open,
    )
    .expect_err("factory error must abort");
    assert!(err.contains("OPENROUTER_API_KEY"));
}

/// A mid-trial transport Http error (retired slug 404, auth 401, outage
/// 5xx) ends the cell as a recorded row with no fault attribution —
/// never an `Err` that aborts the rotation and discards every prior
/// trial row (run 37987255771: one 404 from a retired `:free` candidate
/// left a 0-byte artifact behind a green GHA).
#[test]
fn rotation_contains_mid_trial_http_error_as_row_not_abort() {
    let mut factory = fake_factory(|| {
        Err(TransportError::Http(
            "http 404: model unavailable for free".into(),
        ))
    });
    let scenarios = [probe_scenario()];
    let events = run_rotation(
        &scenarios,
        &["dead/model:free".to_owned()],
        2,
        &mut factory,
        &default_bounds(),
        &budget_open,
    )
    .expect("mid-trial transport error must not abort the rotation");

    let mut rows = 0;
    for event in &events {
        match event {
            RotationEvent::Trial { report } => {
                rows += 1;
                assert_eq!(report.status, TrialStatus::HttpError);
                assert!(report.checks.is_empty(), "no fault attribution");
            }
            RotationEvent::Absent { .. } => panic!("dead model records rows, not absent"),
        }
    }
    assert_eq!(rows, 2, "each cell records its own row");
}

// ── action protocol ──────────────────────────────────────────────────────────

/// A parseable action is executed exactly once, with `executed: true`, and
/// the observation is fed back in the next turn's context.
#[test]
fn parseable_action_executed_once_and_observed() {
    let mut transport = FakeTransport::new(vec![
        Ok(action("printf ok > touched.txt", false)),
        Ok(action("true", true)),
    ]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &default_bounds(),
    )
    .expect("trial runs");

    assert_eq!(report.status, TrialStatus::Passed);
    // Executed exactly once: two actions, two steps, both executed.
    assert_eq!(report.replay.len(), 2);
    assert!(report.replay.iter().all(|s| s.executed));
    assert!(
        report
            .replay
            .iter()
            .any(|s| s.command == "printf ok > touched.txt")
    );
    // The second turn's context carried the observation of the first step.
    let second_turn = &transport.seen_turns[1];
    let observation = second_turn
        .iter()
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        observation.contains("touched.txt") || observation.contains("exit"),
        "observation must be fed back, got: {observation}"
    );
}

/// Malformed output triggers exactly one re-ask with the format error
/// attached; the re-ask consumes a turn of the cap.
#[test]
fn malformed_output_reasked_once_with_error() {
    let mut transport = FakeTransport::new(vec![
        Ok("I will run the fix now".into()), // not an action
        Ok(action("printf ok > touched.txt", true)),
    ]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &default_bounds(),
    )
    .expect("trial runs");

    // Trial still passes: the re-ask recovered.
    assert_eq!(report.status, TrialStatus::Passed);
    // The re-ask turn contains the format error text.
    let reask = transport.seen_turns[1]
        .iter()
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        reask.to_lowercase().contains("format"),
        "re-ask must attach the format error, got: {reask}"
    );
}

/// Persistent malformation ends the trial `invalid_output` with
/// ERR_ACTION_FORMAT_VIOLATION as an agent fault.
#[test]
fn persistent_malformation_is_invalid_output() {
    let mut transport = FakeTransport::new(vec![Ok("nope".into()), Ok("still nope".into())]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &default_bounds(),
    )
    .expect("trial runs");

    assert_eq!(report.status, TrialStatus::InvalidOutput);
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(
        rendered.contains("ERR_ACTION_FORMAT_VIOLATION"),
        "agent fault code must appear: {rendered}"
    );
}

// ── bounds ───────────────────────────────────────────────────────────────────

/// Exceeding the turn cap ends the trial failed with the bound named and
/// no fault attribution.
#[test]
fn turn_cap_stops_trial_with_bound_named() {
    let mut transport =
        FakeTransport::new(vec![Ok(action("true", false)), Ok(action("true", false))]);
    let bounds = Bounds {
        turn_cap: 2,
        ..Bounds::default()
    };
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &bounds,
    )
    .expect("trial runs");

    assert_eq!(report.status, TrialStatus::Failed);
    assert_eq!(report.reached_bound.as_deref(), Some("turn_cap"));
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(
        !rendered.contains("ERR_"),
        "bound stop carries no fault code: {rendered}"
    );
}

/// The token-estimate budget (chars/4) stops runaway traffic with the
/// bound named.
#[test]
fn token_budget_stops_trial_with_bound_named() {
    let mut transport = FakeTransport::new(vec![Ok(action("true", false))]);
    // The initial context (prompt + system + user) already exceeds this.
    let bounds = Bounds {
        token_budget_chars: 50,
        ..Bounds::default()
    };
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &bounds,
    )
    .expect("trial runs");

    assert_eq!(report.status, TrialStatus::Failed);
    assert_eq!(report.reached_bound.as_deref(), Some("token_budget"));
}

/// The wall-clock per-command timeout terminates a runaway process and
/// records it as a distinct step.
#[test]
fn command_timeout_terminates_runaway() {
    let mut transport = FakeTransport::new(vec![
        Ok(action("sleep 5", true)), // runaway command, done anyway
    ]);
    let bounds = Bounds {
        command_timeout_ms: 300,
        ..Bounds::default()
    };
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &bounds,
    )
    .expect("trial runs");

    assert!(
        report.replay[0].stderr.contains("timeout") || report.timed_out_step.is_some(),
        "timeout must be recorded: {report:?}"
    );
}

// ── 429 semantics ────────────────────────────────────────────────────────────

/// One 429 is retried (after the recorded delay) and the trial proceeds.
#[test]
fn single_429_retried_then_proceeds() {
    let mut transport = FakeTransport::new(vec![
        Err(TransportError::RateLimited),
        Ok(action("printf ok > touched.txt", true)),
    ]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &default_bounds(),
    )
    .expect("trial runs");

    assert_eq!(report.status, TrialStatus::Passed);
    assert!(
        report.rate_limit_delay_ms.is_some(),
        "applied 429 delay must be recorded: {report:?}"
    );
}

/// A second consecutive 429 ends the trial rate_limited with no fault.
#[test]
fn second_429_ends_rate_limited() {
    let mut transport = FakeTransport::new(vec![
        Err(TransportError::RateLimited),
        Err(TransportError::RateLimited),
    ]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &default_bounds(),
    )
    .expect("trial runs");

    assert_eq!(report.status, TrialStatus::RateLimited);
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(
        !rendered.contains("ERR_"),
        "rate_limited carries no fault code: {rendered}"
    );
}

// ── report contract ──────────────────────────────────────────────────────────

/// Tier-2 rows carry the raw model id verbatim and the repetition index;
/// the bounds applied are recorded with their values.
#[test]
fn report_row_carries_attribution_and_bounds() {
    let mut transport = FakeTransport::new(vec![Ok(action("true", true))]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        2,
        &mut transport,
        &Bounds {
            turn_cap: 7,
            ..Bounds::default()
        },
    )
    .expect("trial runs");

    let rendered: serde_json::Value = serde_json::to_value(&report).unwrap();
    assert_eq!(
        rendered["model"], "deepseek/deepseek-v4-flash:free",
        "model id verbatim including :free"
    );
    assert_eq!(rendered["repetition"], 2);
    assert_eq!(rendered["report_version"], 1);
    assert_eq!(rendered["bounds"]["turn_cap"], 7);
    assert!(
        rendered["bounds"]["command_timeout_ms"].is_number(),
        "applied bounds recorded with values: {rendered}"
    );
    // Per-check outcomes present (passing checks included).
    assert!(
        rendered["checks"].as_array().unwrap().len() >= 1,
        "one entry per declared check: {rendered}"
    );
}

/// The turn context never includes check logic — only the prompt and the
/// fixture root path.
#[test]
fn turn_context_carries_only_prompt_and_fixture_path() {
    let mut transport = FakeTransport::new(vec![Ok(action("printf ok > touched.txt", true))]);
    let report = run_trial(
        &probe_scenario(),
        "deepseek/deepseek-v4-flash:free",
        0,
        &mut transport,
        &default_bounds(),
    )
    .expect("trial runs");
    assert_eq!(report.status, TrialStatus::Passed);

    for turn in &transport.seen_turns {
        let joined = turn
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !joined.contains("tool_fault") && !joined.contains("touched.txt missing"),
            "check logic must not leak into turn context: {joined}"
        );
        assert!(
            joined.contains("create the file"),
            "scenario prompt must be present: {joined}"
        );
    }
}
