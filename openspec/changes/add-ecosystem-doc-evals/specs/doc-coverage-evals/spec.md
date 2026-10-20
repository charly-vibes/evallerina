# Delta: doc-coverage-evals

## ADDED Requirements

### Requirement: Static doc-coverage audit

The harness SHALL provide a tier-0 `doc-coverage` audit that extracts each
fixture-provisioned tool binary's observable CLI surface (subcommands, long
flags, documented exit codes) and diffs it against feature tokens extracted
from the repo's doc surfaces (`README.md`, `docs/**`, `llms.txt`, `AGENTS.md`
managed blocks).

#### Scenario: Undocumented subcommand detected

- **WHEN** the audited binary's `--help` lists a subcommand whose normalized
  name appears in none of the scanned doc surfaces
- **THEN** the audit emits a row with fault class `ERR_FEATURE_UNDOCUMENTED`
  naming the subcommand and the surfaces scanned

#### Scenario: Audit tolerates absent surfaces

- **WHEN** a tool's fixture declares `surface: none` or a doc surface is
  missing from the fixture repo
- **THEN** the audit skips that source without error and records the skip in
  the audit row's provenance

### Requirement: Doc-findability audit

The harness SHALL distinguish features that are documented nowhere from
features documented but unreachable from the AIX breadcrumb channels: every
feature present in docs but not referenced (directly or via breadcrumb) by
`llms.txt` or an AGENTS.md managed block SHALL be reported as
`ERR_DOC_NOT_FINDABLE`.

#### Scenario: Documented but unfindable feature detected

- **WHEN** the audited binary's `--help` lists a subcommand whose normalized
  name appears in none of the scanned doc surfaces
- **THEN** the audit emits a row with fault class `ERR_FEATURE_UNDOCUMENTED`
  naming the subcommand and the surfaces scanned

#### Scenario: Documented but unfindable feature detected

- **WHEN** a feature token appears in `docs/**` but is not referenced
  (directly or via breadcrumb) by `llms.txt` or any AGENTS.md managed block
- **THEN** the audit emits a row with fault class `ERR_DOC_NOT_FINDABLE`

#### Scenario: Audit tolerates absent surfaces

- **WHEN** a tool's fixture declares `surface: none` or a doc surface is
  missing from the fixture repo
- **THEN** the audit skips that source without error and records the skip in
  the audit row's provenance

### Requirement: Ecosystem-wide routing map parity

The harness SHALL maintain `routing.toml` entries for every tool listed in
dulce-de-leche's MANAGED-TOOLS block plus `ddl`, `genesis`, and
`espectacular`, enforced by a tier-0 `routing-parity` check. Third-party
upstream entries SHALL be marked `upstream = true` and excluded from filing.
When the ddl sibling checkout is absent, the check SHALL degrade to a warning.

#### Scenario: Managed tool missing from routing.toml

- **WHEN** the MANAGED-TOOLS block lists a tool with no `[tool]` section in
  routing.toml
- **THEN** the routing-parity check fails naming the unmapped tool

#### Scenario: Stale repo path detected

- **WHEN** a routing.toml repo path does not resolve from the repo root
- **THEN** the routing-parity check fails naming the tool and the bad path

### Requirement: Repo resolution fallback

Tool → repo resolution for parity entries SHALL use ddl's canonical-names
table when the tool is listed there, falling back to `repo == binary name`
otherwise, overridable by an explicit routing.toml entry.

#### Scenario: Name-mismatch tool resolved from canonical table

- **WHEN** the parity check resolves a tool listed in ddl's canonical-names
  table (e.g. `turu` → `charly-vibes/whisper`)
- **THEN** the resolved repo is the table's repository column, not the binary
  name

#### Scenario: Unlisted tool falls back to binary name

- **WHEN** the parity check resolves a first-party tool absent from the
  canonical-names table (e.g. `testaruda`)
- **THEN** the resolved repo is the binary name, unless routing.toml carries
  an explicit override

#### Scenario: Third-party upstream never filed

- **WHEN** a routed proposal targets an `upstream = true` tool (e.g. `bd`,
  `openspec`) and the operator runs with `--file`
- **THEN** the decision is `skipped` with reason `unmapped_upstream`; no
  ticket is created in the third-party repository

### Requirement: Doc-findability eval family

The harness SHALL support `doc-findability` scenarios scored on process
boundaries only: within a bounded step budget the transcript references the
seeded feature's doc path (located via llms.txt breadcrumbs or AGENTS.md
managed blocks) and a subsequent command succeeds against the documented
interface. Scenario failures SHALL attribute `ERR_DOC_NOT_FINDABLE` to the
faulting tool's repository (doc-authoring fault), never to the genesis
channel work.

#### Scenario: Agent finds feature via breadcrumb

- **WHEN** a replayed (tier-1) or live (tier-2) transcript locates the seeded
  feature's doc path via llms.txt or a managed block and the follow-up command
  succeeds
- **THEN** the scenario scores pass and the row records the discovery channel
  used

#### Scenario: Agent fails to find feature within budget

- **WHEN** the transcript exhausts the step budget without referencing the
  correct doc path
- **THEN** the scenario scores fail with fault class `ERR_DOC_NOT_FINDABLE`
  routed (per the cross-model threshold) to a ticket proposal in the tool's
  mapped repository

### Requirement: Edge-exploration eval family

The harness SHALL support `edge-exploration` scenarios asserting only on
envelope channels, with faults classified exactly per the existing taxonomy:
`ERR_EDGE_UNHANDLED` is a tool fault; a hint the agent ignores reuses the
existing `ERR_ENVELOPE_HINT_BLINDNESS` class as an agent fault.

#### Scenario: Undocumented input fails blind

- **WHEN** an edge input causes a non-zero exit whose stderr/stdout envelope
  contains no `hints` entry
- **THEN** the row classifies `ERR_EDGE_UNHANDLED` and routes a proposal to
  the tool's mapped repository

#### Scenario: Hint emitted but ignored

- **WHEN** the tool emits a `hints` entry for the failing input and the agent
  repeats the identical failing command
- **THEN** the row classifies `ERR_ENVELOPE_HINT_BLINDNESS` and does not
  propose a tool ticket

### Requirement: Fault routing reuses existing machinery

Doc-coverage and edge-exploration faults SHALL flow through the existing
fault-routing path (add-fault-ticket-routing): fingerprint token
`evallerina:<scenario>:<fault-class>`, dry-run proposals by default, filing
only behind explicit `--file` authorization.

#### Scenario: Undocumented feature files ticket in tool repo

- **WHEN** `ERR_FEATURE_UNDOCUMENTED` for scenario `S` against tool `T` is
  observed in trial rows from at least the threshold number of distinct models
  and the operator runs `evallerina report --route --file`
- **THEN** a ticket bearing token `evallerina:S:ERR_FEATURE_UNDOCUMENTED` is
  created in `T`'s mapped repository, deduplicated against open issues

#### Scenario: Dry-run remains the default

- **WHEN** `evallerina report --route` runs without `--file`
- **THEN** every decision is `dry_run` or `skipped`; no `bd create` occurs
