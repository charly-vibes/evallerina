//! Purpose: Tier-2 live runner implementing the evals-guidelines Live
//! action protocol.
//! Responsibilities: run one live trial (scenario × model × repetition)
//! against a [`Transport`]: build per-turn context (prompt + fixture
//! root only), parse one structured action per turn, re-ask once on
//! malformed output, execute parseable actions exactly once inside a
//! sandboxed fixture, enforce bounds (turn cap, wall-clock, token
//! budget), handle 429 with one recorded retry, and produce a versioned
//! tier-2 report row with verbatim model attribution.
//! Rationale: the evals-guidelines Live action protocol + free-tier
//! attribution + sandbox confinement + interoperable report contract;
//! the transport is a seam so the protocol is fully testable offline.

use genesis::evals::{AgentStep, CheckOutcome, ErrorTaxonomy, Scenario, ScenarioResult};

/// A serializable mirror of `genesis::evals::AgentStep` for tier-2
/// report rows (genesis's replay types are not serde-derived). Field
/// names match so the row can be re-loaded as a recorded trajectory.
#[derive(Debug, Clone, Serialize)]
pub struct ReplayStep {
    /// Command line as issued by the agent.
    pub command: String,
    /// Captured stdout of the invocation.
    pub stdout: String,
    /// Captured stderr of the invocation.
    pub stderr: String,
    /// Exit code of the invocation.
    pub exit_code: i32,
    /// Whether the subprocess was actually observed.
    pub executed: bool,
}
use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Version of the tier-2 report row shape. Additive evolution only
/// within a version; breaking changes bump this.
pub const REPORT_VERSION: u32 = 1;

/// Per-model transport builder for rotations: one transport per model
/// cell (the production adapter binds a single model id for its
/// lifetime).
pub type TransportFactory<'a> = &'a mut dyn FnMut(&str) -> Result<Box<dyn Transport>, String>;

/// Applied bounds for a trial (evals-guidelines: bounded live trials).
#[derive(Debug, Clone, Serialize)]
pub struct Bounds {
    /// Maximum turns, re-asks included.
    pub turn_cap: usize,
    /// Wall-clock limit per executed command, milliseconds.
    pub command_timeout_ms: u64,
    /// Token-estimate budget over the trial's full prompt and completion
    /// traffic, using the `aix` chars/4 heuristic (labeled heuristic).
    pub token_budget_chars: usize,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            turn_cap: 10,
            command_timeout_ms: 120_000,
            token_budget_chars: 50_000,
        }
    }
}

/// One message in the per-turn model context.
#[derive(Debug, Clone)]
pub struct Message {
    /// Sender role (`system` | `user` | `assistant`).
    pub role: String,
    /// Message content.
    pub content: String,
}

/// A parsed structured action: exact command line plus done flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// The exact command line to execute.
    pub command: String,
    /// True when the agent declares the task complete after this action.
    pub done: bool,
}

/// Parse one structured action. The format is strict JSON:
/// `{"command": "...", "done": true|false}`.
pub fn parse_action(raw: &str) -> Result<Action, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("action is not JSON: {e}"))?;
    let command = value
        .get("command")
        .and_then(serde_json::Value::as_str)
        .ok_or("action lacks a string `command` field")?
        .to_owned();
    let done = value
        .get("done")
        .and_then(serde_json::Value::as_bool)
        .ok_or("action lacks a boolean `done` field")?;
    if command.trim().is_empty() {
        return Err("action `command` is empty".into());
    }
    Ok(Action { command, done })
}

/// Errors from the model transport. `RateLimited` maps to HTTP 429.
#[derive(Debug)]
pub enum TransportError {
    /// HTTP 429: retried once, then the trial ends `rate_limited`.
    RateLimited,
    /// Any other transport failure (network, auth, 5xx). Tool-side.
    Http(String),
}

/// Model transport seam. The production adapter is the OpenRouter
/// chat-completions client; tests script a fake.
pub trait Transport {
    /// Send the full message list, return the completion text.
    fn complete(&mut self, messages: &[Message]) -> Result<String, TransportError>;
}

/// AIX ablation arm (evals.md Step 5): the environment variant a trial
/// runs in. `Full` arms carry the AIX artifacts (llms.txt, managed
/// AGENTS.md, `.genesis/tools.toml`); `Ablated` arms expose raw
/// binaries with no AIX context. Serialized on tier-2 rows — an
/// additive field, report_version 1 unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arm {
    /// AIX artifacts provisioned (llms.txt, managed AGENTS.md,
    /// `.genesis/tools.toml`).
    Full,
    /// Raw binaries, no AIX context.
    Ablated,
}

/// Trial outcome status (evals-guidelines interoperable report contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialStatus {
    /// Every declared check passed.
    Passed,
    /// A check failed, or a bound was reached.
    Failed,
    /// Model transport 429 twice; no fault attribution.
    RateLimited,
    /// Persistent action malformation; carries
    /// `ERR_ACTION_FORMAT_VIOLATION` as an agent fault.
    InvalidOutput,
}

/// One per-check outcome in a tier-2 report row.
#[derive(Debug, Clone, Serialize)]
pub struct CheckRow {
    /// Declared check name.
    pub name: String,
    /// true when the check passed.
    pub passed: bool,
    /// Agent-fault classification, present when and only when the
    /// failure is an agent fault (omitted otherwise, never null).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<&'static str>,
    /// Human-readable reason (for humans and tickets, not parsed by
    /// aggregators). Present for failures, omitted for passes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One event of a rotation run over the registry: either a completed
/// trial report row, or an absent record for a cell the per-run
/// wall-clock budget skipped (evals-guidelines: skipped cells are
/// recorded as absent, never failed, never silently dropped).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RotationEvent {
    /// A trial ran; carries the full report_version 1 row.
    Trial {
        /// The trial's report row.
        #[serde(flatten)]
        report: Box<TrialReport>,
    },
    /// A trial did not run: the per-run budget was exhausted before it
    /// could start. No fault attribution; the row is the record.
    Absent {
        /// Scenario name.
        scenario: String,
        /// Raw model id, verbatim including `:free`.
        model: String,
        /// 0-based repetition index that was skipped.
        repetition: u32,
        /// The environment variant the cell belongs to (A/B ablation).
        arm: Arm,
    },
}

/// A tier-2 trial report row (report_version 1).
#[derive(Debug, Clone, Serialize)]
pub struct TrialReport {
    /// Report shape version.
    pub report_version: u32,
    /// Scenario name.
    pub scenario: String,
    /// Tool under eval (the registry of tools lives with the scenarios).
    pub tool: String,
    /// Trial outcome status.
    pub status: TrialStatus,
    /// Agent-fault code carried by the status, when the status implies
    /// one (`invalid_output` → `ERR_ACTION_FORMAT_VIOLATION`; bound stops
    /// and `rate_limited` carry none). Omitted, never null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault_code: Option<&'static str>,
    /// The replayed live trajectory (feed for offline re-replay).
    pub replay: Vec<ReplayStep>,
    /// One entry per declared check, passing and failing.
    pub checks: Vec<CheckRow>,
    /// The applied bounds with their values.
    pub bounds: Bounds,
    /// The bound that stopped the trial, when a bound was reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reached_bound: Option<&'static str>,
    /// The delay applied before the single 429 retry, when one occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_delay_ms: Option<u64>,
    /// The raw model id, verbatim including `:free`.
    pub model: String,
    /// 0-based repetition index within the scenario × model cell.
    pub repetition: u32,
    /// The environment variant the trial ran in (A/B ablation,
    /// evals.md Step 5). Additive field, report_version 1 unchanged.
    pub arm: Arm,
    /// Step terminated by the per-command wall-clock timeout, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_out_step: Option<usize>,
    /// Applied sandbox isolation, recorded honestly.
    pub sandbox: SandboxReport,
    /// Token-estimate usage (chars/4 heuristic, labeled heuristic).
    pub token_estimate: usize,
}

/// Applied sandbox isolation for a trial.
#[derive(Debug, Clone, Serialize)]
pub struct SandboxReport {
    /// Isolated HOME directory was used.
    pub isolated_home: bool,
    /// Environment was scrubbed to a minimal allowlist.
    pub scrubbed_env: bool,
    /// Network isolation mechanism actually applied.
    pub network_isolation: &'static str,
}

///chars/4 token estimate (aix heuristic, labeled heuristic in reports).
fn estimate_tokens(chars: usize) -> usize {
    chars / 4
}

/// OpenRouter chat-completions transport (production adapter).
/// Thin by contract: one endpoint, bearer auth, raw completion text.
/// Requires `OPENROUTER_API_KEY`; never used in offline tests.
pub struct OpenRouterTransport {
    agent: ureq::Agent,
    api_key: String,
    model: String,
}

impl OpenRouterTransport {
    /// Build a transport for one model cell.
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .build(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Build from the environment; the harness refuses to start tier-2
    /// trials without a key (the anti-goal is silent degradation, not a
    /// hidden key).
    pub fn from_env(model: impl Into<String>) -> Result<Self, String> {
        let key = std::env::var("OPENROUTER_API_KEY").map_err(|_| {
            "OPENROUTER_API_KEY is not set — tier-2 live trials require it".to_owned()
        })?;
        Ok(Self::new(model, key))
    }
}

impl Transport for OpenRouterTransport {
    fn complete(&mut self, messages: &[Message]) -> Result<String, TransportError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages
                .iter()
                .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
                .collect::<Vec<_>>(),
        });
        match self
            .agent
            .post("https://openrouter.ai/api/v1/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(body)
        {
            Ok(resp) => {
                let value: serde_json::Value = resp
                    .into_json()
                    .map_err(|e| TransportError::Http(format!("decode: {e}")))?;
                let content = value
                    .pointer("/choices/0/message/content")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        TransportError::Http(format!("no choices[0].message.content: {value}"))
                    })?;
                Ok(content.to_owned())
            }
            Err(ureq::Error::Status(429, _)) => Err(TransportError::RateLimited),
            Err(ureq::Error::Status(code, resp)) => Err(TransportError::Http(format!(
                "http {code}: {}",
                resp.into_string().unwrap_or_default()
            ))),
            Err(e) => Err(TransportError::Http(e.to_string())),
        }
    }
}

/// Execute one command inside the sandbox: fixture root as cwd, isolated
/// HOME, scrubbed env, wall-clock timeout. Returns the recorded step and
/// whether the step was terminated by the timeout.
fn execute_sandboxed(
    fixture_root: &std::path::Path,
    command: &str,
    timeout: Duration,
    home: &std::path::Path,
) -> (ReplayStep, bool) {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command).current_dir(fixture_root);
    // Scrubbed environment: minimal allowlist. Isolated HOME.
    cmd.env_clear();
    cmd.env("HOME", home);
    cmd.env(
        "PATH",
        "/usr/local/bin:/usr/bin:/bin:/run/current-system/sw/bin",
    );
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let Ok(mut child) = cmd.spawn() else {
        return (
            ReplayStep {
                command: command.to_owned(),
                stdout: String::new(),
                stderr: "sandbox: failed to spawn command".into(),
                exit_code: 127,
                executed: true,
            },
            false,
        );
    };
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_string(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    use std::io::Read;
                    let _ = err.read_to_string(&mut stderr);
                }
                let step = ReplayStep {
                    command: command.to_owned(),
                    stdout,
                    stderr,
                    exit_code: status.code().unwrap_or(-1),
                    executed: true,
                };
                return (step, timed_out);
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    timed_out = true;
                    let _ = child.kill();
                    let _ = child.wait();
                    return (
                        ReplayStep {
                            command: command.to_owned(),
                            stdout: String::new(),
                            stderr: format!(
                                "command terminated by the {}ms wall-clock timeout",
                                timeout.as_millis()
                            ),
                            exit_code: -1,
                            executed: true,
                        },
                        timed_out,
                    );
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                return (
                    ReplayStep {
                        command: command.to_owned(),
                        stdout: String::new(),
                        stderr: "sandbox: wait failed".into(),
                        exit_code: -1,
                        executed: true,
                    },
                    false,
                );
            }
        }
    }
}

/// Run one live trial: scenario × model × repetition.
///
/// Protocol per evals-guidelines:
/// - per-turn context carries only the scenario prompt and the fixture
///   root path inside the sandbox (never check logic or distractors);
/// - one structured action per turn; malformed output re-asked once
///   with the format error attached (re-ask consumes a turn); persistent
///   malformation ends `invalid_output` with
///   `ERR_ACTION_FORMAT_VIOLATION`;
/// - parseable actions are executed exactly once, `executed: true`;
/// - bounds (turn cap, wall-clock, token budget) stop the trial
///   `failed` with the bound named and no fault attribution;
/// - HTTP 429 is retried once after a recorded ≥2s delay; a second 429
///   ends `rate_limited` with no fault attribution.
#[allow(clippy::too_many_lines)]
pub fn run_trial(
    scenario: &Scenario,
    model: &str,
    repetition: u32,
    transport: &mut dyn Transport,
    bounds: &Bounds,
) -> Result<TrialReport, String> {
    run_trial_with_arm(
        scenario,
        model,
        repetition,
        transport,
        bounds,
        Arm::Full,
        &[],
    )
}

/// [`run_trial`] with an A/B ablation arm (evals.md Step 5): `arm` is
/// recorded on the row, and `extra_fixture_files` are written into the
/// fixture after the scenario's own fixtures and distractors (the full
/// arm provisions its AIX artifacts this way; the ablated arm passes
/// none). Extra files never overwrite scenario material — callers
/// must pre-check collisions.
pub fn run_trial_with_arm(
    scenario: &Scenario,
    model: &str,
    repetition: u32,
    transport: &mut dyn Transport,
    bounds: &Bounds,
    arm: Arm,
    extra_fixture_files: &[(String, String)],
) -> Result<TrialReport, String> {
    // Materialize the fixture once; the agent observes it only through
    // executed actions.
    let fixture = genesis::fixture::Fixture::new()
        .with_marker(".sandbox-home")
        .build()
        .map_err(|e| format!("fixture: {e}"))?;
    for (path, content) in &scenario.fixture_files {
        write_fixture_file(fixture.root(), path, content)?;
    }
    for d in &scenario.distractors {
        write_fixture_file(fixture.root(), &d.path, &d.content)?;
    }
    for (path, content) in extra_fixture_files {
        write_fixture_file(fixture.root(), path, content)?;
    }
    let fixture_root: PathBuf = fixture.root().to_path_buf();
    let home = fixture.root().join(".sandbox-home");
    std::fs::create_dir_all(&home).map_err(|e| format!("sandbox home: {e}"))?;

    let mut replay: Vec<ReplayStep> = Vec::new();
    let mut timed_out_step: Option<usize> = None;
    let mut rate_limit_delay_ms: Option<u64> = None;
    let mut traffic_chars: usize = scenario.prompt.chars().count();
    let mut reached_bound: Option<&'static str> = None;
    // Terminal status set inside the loop for rate-limits, bound stops,
    // and persistent malformation; when the loop exits via `done`, the
    // deterministic checks decide between Passed and Failed.
    let mut terminal: Option<TrialStatus> = None;

    // Turn context: scenario prompt + fixture root path ONLY.
    let system = Message {
        role: "system".into(),
        content: "You are an autonomous agent. Respond with exactly one JSON action per turn: {\"command\": \"<exact shell command>\", \"done\": <true|false>}. No prose, no markdown fences. Set done=true when the task is complete."
            .to_owned(),
    };
    let user = Message {
        role: "user".into(),
        content: format!(
            "{}\n\nSandbox fixture root: {}",
            scenario.prompt,
            fixture_root.display()
        ),
    };
    let mut messages: Vec<Message> = vec![system, user];
    traffic_chars += messages[0].content.chars().count() + messages[1].content.chars().count();

    let mut turns_used: usize = 0;
    let mut reasked: bool = false;

    loop {
        // Bounds: turn cap first (re-asks consume turns).
        if turns_used >= bounds.turn_cap {
            reached_bound = Some("turn_cap");
            terminal = Some(TrialStatus::Failed);
            break;
        }
        if estimate_tokens(traffic_chars) >= bounds.token_budget_chars {
            reached_bound = Some("token_budget");
            terminal = Some(TrialStatus::Failed);
            break;
        }

        let completion = match transport.complete(&messages) {
            Ok(text) => {
                traffic_chars += text.chars().count();
                text
            }
            Err(TransportError::RateLimited) => {
                // One retry after a recorded delay of at least two seconds.
                let delay_ms = rate_limit_delay_ms.unwrap_or(2_000);
                if rate_limit_delay_ms.is_some() {
                    // Second consecutive 429 — trial ends rate_limited,
                    // never attributed as a fault, never a different model.
                    terminal = Some(TrialStatus::RateLimited);
                    break;
                }
                std::thread::sleep(Duration::from_millis(delay_ms));
                rate_limit_delay_ms = Some(delay_ms);
                match transport.complete(&messages) {
                    Ok(text) => {
                        traffic_chars += text.chars().count();
                        text
                    }
                    Err(TransportError::RateLimited) => {
                        terminal = Some(TrialStatus::RateLimited);
                        break;
                    }
                    Err(TransportError::Http(e)) => {
                        return Err(format!("transport: {e}"));
                    }
                }
            }
            Err(TransportError::Http(e)) => return Err(format!("transport: {e}")),
        };
        turns_used += 1;

        match parse_action(&completion) {
            Ok(action) => {
                reasked = false;
                // Execute the action exactly once, inside the sandbox.
                let (step, timed_out) = execute_sandboxed(
                    &fixture_root,
                    &action.command,
                    Duration::from_millis(bounds.command_timeout_ms),
                    &home,
                );
                if timed_out {
                    timed_out_step = Some(replay.len());
                }
                traffic_chars += step.stdout.chars().count() + step.stderr.chars().count();
                let observation = format!(
                    "exit: {}\nstdout:\n{}\nstderr:\n{}",
                    step.exit_code, step.stdout, step.stderr
                );
                messages.push(Message {
                    role: "assistant".into(),
                    content: completion.clone(),
                });
                traffic_chars += observation.chars().count();
                messages.push(Message {
                    role: "user".into(),
                    content: observation,
                });
                replay.push(step);
                if action.done {
                    break;
                }
            }
            Err(format_error) => {
                if reasked {
                    // Persistent malformation: agent fault.
                    terminal = Some(TrialStatus::InvalidOutput);
                    break;
                }
                reasked = true;
                turns_used += 1; // the re-ask consumes a turn of the cap
                messages.push(Message {
                    role: "assistant".into(),
                    content: completion.clone(),
                });
                messages.push(Message {
                    role: "user".into(),
                    content: format!(
                        "FORMAT ERROR: {format_error}. Respond with exactly one JSON \
                         action: {{\"command\": \"<exact shell command>\", \"done\": \
                         <true|false>}}. No prose, no markdown fences."
                    ),
                });
                traffic_chars += format_error.chars().count() + 40;
            }
        }
    }

    // Deterministic checks over the live replay.
    let result = ScenarioResult {
        steps: replay
            .iter()
            .map(|s| AgentStep {
                command: s.command.clone(),
                stdout: s.stdout.clone(),
                stderr: s.stderr.clone(),
                exit_code: s.exit_code,
                executed: s.executed,
            })
            .collect(),
        fixture_root: fixture_root.clone(),
        distractors: scenario.distractors.clone(),
    };
    let mut checks = Vec::new();
    let mut all_passed = true;
    for check in &scenario.checks {
        let outcome = (check.check)(&result);
        let row = match outcome {
            CheckOutcome::Pass => {
                checks.push(CheckRow {
                    name: check.name.clone(),
                    passed: true,
                    code: None,
                    reason: None,
                });
                continue;
            }
            CheckOutcome::Fail { taxonomy, reason } => (taxonomy, reason),
        };
        all_passed = false;
        checks.push(CheckRow {
            name: check.name.clone(),
            passed: false,
            code: row.0.as_ref().map(ErrorTaxonomy::code),
            reason: Some(row.1),
        });
    }
    // A done-flag exit (no terminal status) is decided by the checks.
    let status = terminal.unwrap_or(if all_passed {
        TrialStatus::Passed
    } else {
        TrialStatus::Failed
    });

    let fault_code = match status {
        TrialStatus::InvalidOutput => Some("ERR_ACTION_FORMAT_VIOLATION"),
        _ => None,
    };

    Ok(TrialReport {
        report_version: REPORT_VERSION,
        scenario: scenario.name.clone(),
        tool: "wai".into(),
        status,
        fault_code,
        replay,
        checks,
        bounds: bounds.clone(),
        reached_bound,
        rate_limit_delay_ms,
        model: model.to_owned(),
        repetition,
        arm,
        timed_out_step,
        sandbox: SandboxReport {
            isolated_home: true,
            scrubbed_env: true,
            // Best-effort: true network denial requires an external
            // sandbox (namespaces/VM); recorded honestly here.
            network_isolation: "not_enforced_in_process",
        },
        token_estimate: estimate_tokens(traffic_chars),
    })
}

/// Run a rotation over scenario × model cells: the model registry stays
/// the outer loop (in scheduling order, so `:free` ids run first across
/// the whole scenario set), every scenario × repetition is a live trial
/// — unless the per-run budget is exhausted first, in which case the
/// remaining trials are recorded as [`RotationEvent::Absent`] (never
/// failed, never dropped). A transport-factory error aborts the whole
/// rotation loudly. The budget is a closure (checked before each trial)
/// so callers can enforce any per-run policy and tests stay
/// deterministic.
pub fn run_rotation(
    scenarios: &[Scenario],
    models: &[String],
    reps: u32,
    transport_factory: TransportFactory<'_>,
    bounds: &Bounds,
    budget_remaining: &dyn Fn() -> bool,
) -> Result<Vec<RotationEvent>, String> {
    run_rotation_with_arm(
        scenarios,
        models,
        reps,
        &ArmContext::full(),
        transport_factory,
        bounds,
        budget_remaining,
    )
}

/// The A/B arm context a rotation runs under: the arm recorded on the
/// rows plus the arm's extra fixture files (the full arm's AIX
/// artifacts). Bundled so the rotation API stays arg-disciplined.
#[derive(Debug, Clone)]
pub struct ArmContext<'a> {
    /// The arm recorded on every row.
    pub arm: Arm,
    /// Files written into each trial's fixture after the scenario's
    /// own material (empty for the ablated arm).
    pub extra_fixture_files: &'a [(String, String)],
}

impl ArmContext<'_> {
    /// The plain rotation context: full arm, no extra fixtures.
    pub fn full() -> Self {
        Self {
            arm: Arm::Full,
            extra_fixture_files: &[],
        }
    }
}

/// [`run_rotation`] with an A/B ablation arm (evals.md Step 5): every
/// cell is attributed to the context's arm on its row, and its
/// `extra_fixture_files` are provisioned into each trial's fixture
/// after the scenario's own material (the full arm's AIX artifacts).
pub fn run_rotation_with_arm(
    scenarios: &[Scenario],
    models: &[String],
    reps: u32,
    context: &ArmContext<'_>,
    transport_factory: TransportFactory<'_>,
    bounds: &Bounds,
    budget_remaining: &dyn Fn() -> bool,
) -> Result<Vec<RotationEvent>, String> {
    let mut events = Vec::new();
    for model in models {
        // One transport per model cell: the production adapter binds a
        // single model id for its lifetime.
        let mut transport = transport_factory(model)?;
        for scenario in scenarios {
            for repetition in 0..reps {
                if !budget_remaining() {
                    events.push(RotationEvent::Absent {
                        scenario: scenario.name.clone(),
                        model: model.clone(),
                        repetition,
                        arm: context.arm,
                    });
                    continue;
                }
                let report = run_trial_with_arm(
                    scenario,
                    model,
                    repetition,
                    &mut *transport,
                    bounds,
                    context.arm,
                    context.extra_fixture_files,
                )?;
                events.push(RotationEvent::Trial {
                    report: Box::new(report),
                });
            }
        }
    }
    Ok(events)
}

fn write_fixture_file(root: &std::path::Path, path: &str, content: &str) -> Result<(), String> {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("fixture dir: {e}"))?;
    }
    std::fs::write(&target, content).map_err(|e| format!("fixture file: {e}"))
}
