//! Purpose: Envelope readers and deterministic checks for the
//! dulce-de-leche tool family's actual output channels.
//! Responsibilities: extract the JSON envelope from captured stdout
//! (tolerating the tools' habit of printing human preamble lines before
//! the envelope), read remediation hints from `data.remediation` (the
//! channel wai actually emits, versus genesis's generic top-level
//! `hints`), and turn those reads into `CheckOutcome`s.
//! Rationale: one fault per check (evals-guidelines). Each check owns a
//! single signal — envelope shape, hint-following, execution reality,
//! recovery — so a failure names exactly one thing to fix. Tool-shape
//! mismatches are *tool faults* (they become tickets in the tool's
//! repo); agent missteps are *agent faults* with an `ERR_*` taxonomy
//! code.

use genesis::evals::{CheckOutcome, EnvelopeOutcome, ScenarioResult, parse_envelope};
use serde_json::Value;

/// Failure while reading a captured envelope.
#[derive(Debug, thiserror::Error)]
pub enum EnvelopeError {
    /// No JSON object found anywhere in stdout.
    #[error("stdout carries no JSON envelope")]
    NoJson,
    /// The JSON payload is not a valid envelope (missing `ok`).
    #[error("invalid envelope: {0}")]
    Invalid(#[from] genesis::evals::EvalsError),
}

/// Extract the JSON envelope from stdout.
///
/// Dulce tools sometimes print human preamble (e.g. wai's
/// `Using default project name: …`) before the envelope, so the first
/// `{` anywhere in stdout starts the payload — free-form agent text is
/// never scored, only what the tool printed, and this stays a
/// process-boundary read.
pub fn extract_envelope(stdout: &str) -> Result<Value, EnvelopeError> {
    let start = stdout.find('{').ok_or(EnvelopeError::NoJson)?;
    let value: Value = serde_json::from_str(&stdout[start..]).map_err(JsonError)?;
    if value.get("ok").and_then(Value::as_bool).is_none() {
        return Err(EnvelopeError::Invalid(
            genesis::evals::EvalsError::NoOkField,
        ));
    }
    Ok(value)
}

/// Bridge so `serde_json::Error` from extraction maps onto
/// [`EnvelopeError::Invalid`] without duplicating genesis's variants.
struct JsonError(serde_json::Error);
impl From<JsonError> for EnvelopeError {
    fn from(e: JsonError) -> Self {
        EnvelopeError::Invalid(genesis::evals::EvalsError::NotJson(e.0))
    }
}

/// Commands suggested by the envelope, in order.
///
/// Reads the dulce channel `data.remediation[].command` *and* the
/// generic top-level `hints[].command` (genesis helpers' shape), so a
/// check passes against either channel layout. Covering both here —
/// not in a check — keeps each check single-fault.
pub fn remediation_commands(value: &Value) -> Vec<String> {
    let read_commands = |arr: &Vec<Value>| -> Vec<String> {
        arr.iter()
            .filter_map(|h| h.get("command").and_then(Value::as_str))
            .map(str::to_owned)
            .collect()
    };
    let mut commands = value
        .pointer("/data/remediation")
        .and_then(Value::as_array)
        .map(read_commands)
        .unwrap_or_default();
    if let Ok(EnvelopeOutcome::Error {
        mut hint_commands, ..
    }) = parse_envelope(&serde_json::to_string(value).expect("Value serializes"))
    {
        commands.append(&mut hint_commands);
    }
    commands
}

/// Assert `steps[step]` exited nonzero with a parseable error envelope
/// whose remediation suggests `suggested_command`. Tool fault if the
/// envelope is missing, still `ok:true`, or hintless — the channel is
/// the tool's promise, and a broken channel is a tool ticket, never an
/// agent fault.
pub fn error_envelope_with_remediation_hint(
    step: usize,
    suggested_command: impl Into<String>,
) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let suggested = suggested_command.into();
    move |result: &ScenarioResult| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        if s.exit_code == 0 {
            return CheckOutcome::tool_fault(format!(
                "step {step} exited 0; expected a failing invocation"
            ));
        }
        let value = match extract_envelope(&s.stdout) {
            Ok(v) => v,
            Err(e) => return CheckOutcome::tool_fault(format!("step {step}: {e}")),
        };
        if value.get("ok").and_then(Value::as_bool) != Some(false) {
            return CheckOutcome::tool_fault(format!("step {step} is not an error envelope"));
        }
        let commands = remediation_commands(&value);
        if commands.iter().any(|c| c == &suggested) {
            CheckOutcome::pass()
        } else {
            CheckOutcome::tool_fault(format!(
                "step {step} error envelope lacks remediation `{suggested}` (suggested: {commands:?})"
            ))
        }
    }
}

/// Assert `steps[recovery_index]` issued the command suggested by an
/// earlier error envelope. Agent fault (`ERR_ENVELOPE_HINT_BLINDNESS`)
/// otherwise.
///
/// Assert `steps[step]` exited 0 with an `ok:true` envelope, extracted
/// leniently from stdout (preamble tolerated). Tool fault otherwise:
/// the agent may have followed the hint correctly and the fix itself
/// failed, which is the tool's responsibility.
pub fn ok_envelope_loose(step: usize) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    move |result: &ScenarioResult| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        let value = match extract_envelope(&s.stdout) {
            Ok(v) => v,
            Err(e) => return CheckOutcome::tool_fault(format!("step {step}: {e}")),
        };
        if value.get("ok").and_then(Value::as_bool) == Some(true) && s.exit_code == 0 {
            CheckOutcome::pass()
        } else {
            CheckOutcome::tool_fault(format!(
                "step {step}: ok envelope expected, exit {}",
                s.exit_code
            ))
        }
    }
}

/// Assert `steps[step]` exited nonzero and its stderr carries wai's
/// clap-level DidYouMean hint (`Did you mean '<suggested>'?`). Tool
/// fault otherwise. Unlike the envelope channels, the typo channel is
/// plain stderr text with no JSON envelope — the hint string is still a
/// deterministic tool promise, so a substring read (not prose scoring)
/// is the process-boundary check.
pub fn stderr_did_you_mean(
    step: usize,
    suggested_command: impl Into<String>,
) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let suggested = suggested_command.into();
    let needle = format!("Did you mean '{suggested}'?");
    move |result: &ScenarioResult| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        if s.exit_code == 0 {
            return CheckOutcome::tool_fault(format!(
                "step {step} exited 0; expected a failing invocation"
            ));
        }
        if s.stderr.contains(&needle) {
            CheckOutcome::pass()
        } else {
            CheckOutcome::tool_fault(format!("step {step} stderr lacks DidYouMean `{needle}`"))
        }
    }
}

/// Assert `steps[step]` exited nonzero with a parseable error envelope
/// some of whose remediation commands *contain* `fragment`. Tool fault
/// otherwise.
///
/// The substring form exists because dont's remediation commands embed
/// placeholders (`dont trust <id> --reason "<specific grounds>"`) and
/// capture-time literals (the minted claim id) that an agent is not
/// expected to reproduce character-for-character; asserting the stable
/// fragment keeps the check single-fault on hint presence.
pub fn error_envelope_with_hint_containing(
    step: usize,
    fragment: impl Into<String>,
) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let fragment = fragment.into();
    move |result: &ScenarioResult| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        if s.exit_code == 0 {
            return CheckOutcome::tool_fault(format!(
                "step {step} exited 0; expected a failing invocation"
            ));
        }
        let value = match extract_envelope(&s.stdout) {
            Ok(v) => v,
            Err(e) => return CheckOutcome::tool_fault(format!("step {step}: {e}")),
        };
        if value.get("ok").and_then(Value::as_bool) != Some(false) {
            return CheckOutcome::tool_fault(format!("step {step} is not an error envelope"));
        }
        let commands = remediation_commands(&value);
        if commands.iter().any(|c| c.contains(&fragment)) {
            CheckOutcome::pass()
        } else {
            CheckOutcome::tool_fault(format!(
                "step {step} error envelope lacks remediation containing `{fragment}` (suggested: {commands:?})"
            ))
        }
    }
}

/// Assert the envelope at `steps[step]` carries `expected` at JSON
/// `pointer`. Tool fault otherwise (the agent cannot change what the
/// tool reports about state it did not create).
///
/// Generic single-fault state read — e.g. dont's claim lifecycle lands
/// on `data.status == "verified"` after the hint-following path.
pub fn envelope_field_is(
    step: usize,
    pointer: impl Into<String>,
    expected: &str,
) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let pointer = pointer.into();
    let expected = expected.to_owned();
    move |result: &ScenarioResult| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        let value = match extract_envelope(&s.stdout) {
            Ok(v) => v,
            Err(e) => return CheckOutcome::tool_fault(format!("step {step}: {e}")),
        };
        match value.pointer(&pointer).and_then(Value::as_str) {
            Some(actual) if actual == expected => CheckOutcome::pass(),
            other => CheckOutcome::tool_fault(format!(
                "step {step}: expected {pointer} == `{expected}`, got {other:?}"
            )),
        }
    }
}
