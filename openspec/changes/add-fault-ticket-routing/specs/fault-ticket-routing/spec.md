## ADDED Requirements

### Requirement: Tool faults route to tickets in the tool's repo

The system SHALL provide a routing step over tier-2 report rows that maps
tool-fault check outcomes to tickets in the faulting tool's own repository,
using the `bd` CLI executed with the target repository as working directory.
Each routed ticket SHALL contain a fingerprint token
`evallerina:<scenario>:<fault-class>` linking it back to the originating
scenario and fault classification; a fault recurring on a new tool version
reuses the token (version goes in the ticket body, not the token). Proposals
SHALL key the target repository on the scenario definition's recorded `tool`
(the provenance field the scenario carries), not on the report row's `tool`
field, which is currently hardcoded for live trials and may misattribute; the
row's tool value is kept in proposals for provenance only.

#### Scenario: tool fault above threshold becomes a ticket

- **WHEN** a tier-2 report contains a tool fault with the same fault key
  (scenario, fault-class, tool) observed in trial rows from at least the
  routing threshold — min(3, |configured model registry|), where the
  registry is the model list the rotation was configured with — distinct
  models
- **AND** routing is invoked with filing authorized
- **THEN** a `bd create` invocation runs in the repository mapped from the
  scenario's recorded tool
- **AND** the created issue title, description, and token identify the
  scenario, fault class, tool, and report row provenance

#### Scenario: agent faults are never routed

- **WHEN** a tier-2 report contains agent faults (`ERR_*` taxonomy)
- **THEN** no ticket is proposed or filed for them
- **AND** they remain visible in the per-channel fault dashboard only

### Requirement: Routing deduplicates against the target repo's tracker

Before filing, the system SHALL search the target repository's issue tracker
for an open issue containing the fingerprint token, using
`bd search --status open <token>` in the target repository (closed issues
count as no match, so a fault that regresses after its ticket was closed is
re-filed). If found, the decision SHALL be `existing` and no new issue SHALL
be created.

#### Scenario: repeated fault does not duplicate tickets

- **WHEN** an open issue in the target repo already contains the token
  `evallerina:<scenario>:<fault-class>`
- **AND** routing is invoked with filing authorized
- **THEN** the decision recorded for that fault is `existing`
- **AND** no second issue is created

#### Scenario: closed ticket does not block re-filing a regressed fault

- **WHEN** the only issue in the target repo containing the token is closed
- **AND** routing is invoked with filing authorized
- **THEN** a new issue is proposed and filed
- **AND** the decision recorded for that fault is `filed`

### Requirement: Routing decisions are recorded additively in the report

The report SHALL carry a `routed` summary field with per-decision counts
(`filed`, `existing`, `dry_run`, `skipped`). This field is additive;
`report_version` SHALL remain 1.

#### Scenario: dry-run routing produces proposals without writes

- **WHEN** routing is invoked without filing authorized (including all
  scheduled CI runs)
- **THEN** a `proposed-tickets.jsonl` artifact is written next to the report
- **AND** every above-threshold fault has decision `dry_run`
- **AND** no `bd create` invocation is made in any repository

Each proposal row SHALL carry the fields `tool`, `repo`, `scenario`,
`fault_class`, `token`, `decision`, `reason` (null when none), and
`row_provenance` (report row scenario/model/tool values); `bd_issue_id` is
added when a ticket is filed. The schema is machine-checkable so the CI
artifact is reviewable without reading code.

#### Scenario: unmapped tool is skipped, not fatal

- **WHEN** a faulting tool has no entry in the routing mapping
- **THEN** the decision for its faults is `skipped` with the reason recorded
- **AND** the report completes successfully

#### Scenario: filing failure is skipped with reason, not fatal

- **WHEN** filing is authorized but the `bd create` invocation fails in the
  target repository (not a beads repo, `bd` binary missing, repository
  locked)
- **THEN** the decision for that fault is `skipped` with reason `bd_failure`
- **AND** the report completes successfully and records the failure reason
- **AND** no partial count is reported as `filed`
