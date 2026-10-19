## ADDED Requirements

### Requirement: Tool faults route to tickets in the tool's repo

The system SHALL provide a routing step over tier-2 report rows that maps
tool-fault check outcomes to tickets in the faulting tool's own repository,
using the `bd` CLI executed with the target repository as working directory.
Each routed ticket SHALL contain a fingerprint token
`evallerina:<scenario>:<fault-class>` linking it back to the originating
scenario and fault classification.

#### Scenario: tool fault above threshold becomes a ticket

- **WHEN** a tier-2 report contains a tool fault for the same
  (scenario, fault-class, tool) cell in at least the routing threshold
  (min(3, configured registry size)) of recorded cells
- **AND** routing is invoked with filing authorized
- **THEN** a `bd create` invocation runs in the mapped tool repository
- **AND** the created issue title, description, and token identify the
  scenario, fault class, tool, and report row provenance

#### Scenario: agent faults are never routed

- **WHEN** a tier-2 report contains agent faults (`ERR_*` taxonomy)
- **THEN** no ticket is proposed or filed for them
- **AND** they remain visible in the per-channel fault dashboard only

### Requirement: Routing deduplicates against the target repo's tracker

Before filing, the system SHALL search the target repository's issue tracker
for an open issue containing the fingerprint token. If found, the decision
SHALL be `existing` and no new issue SHALL be created.

#### Scenario: repeated fault does not duplicate tickets

- **WHEN** an open issue in the target repo already contains the token
  `evallerina:<scenario>:<fault-class>`
- **AND** routing is invoked with filing authorized
- **THEN** the decision recorded for that fault is `existing`
- **AND** no second issue is created

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

#### Scenario: unmapped tool is skipped, not fatal

- **WHEN** a faulting tool has no entry in the routing mapping
- **THEN** the decision for its faults is `skipped` with the reason recorded
- **AND** the report completes successfully
