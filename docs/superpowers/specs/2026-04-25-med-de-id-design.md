# med-de-id Design Specification

**Date:** 2026-04-25  
**Status:** Draft approved in conversation; written spec for review  
**Product:** `med-de-id`

## 1. Product Definition

`med-de-id` is a Windows-first, local-first medical de-identification platform built around a pure Rust core. It supports reversible de-identification for multiple medical data formats, preserves auditability, and exposes three formal product surfaces:

1. **CLI** — automation, batch, integration, headless execution
2. **Browser tool** — local-first localhost workflow surface for visual flow composition and run control
3. **Desktop app** — sensitive workstation for review, vault operations, decode flows, and audit investigation

The system is not a SaaS product. Processing, vault management, scheduling, and high-risk decode operations are designed to run locally and offline by default.

## 2. Goals

### 2.1 Primary goals

- Detect PHI across major medical file families
- Perform reversible encode/tokenization with secure mapping storage
- Support controlled decode/re-identification when authorized
- Provide a single shared core with multiple format adapters and three interaction surfaces
- Support broad file-family coverage in v1, with explicit depth differences by format
- Support local scheduling and reusable pipelines
- Preserve auditability and governance around sensitive operations

### 2.2 Product positioning

`med-de-id` is not just a parser set and not just a batch utility. It is a governed workflow system with a controlled lifecycle:

`ingest -> extract -> detect -> review -> encode -> export -> decode -> audit`

### 2.3 Non-goals for v1

The following are explicitly out of scope for v1:

- SaaS or cloud-first deployment model
- General-purpose automation platform for arbitrary business workflows
- Full AI/NLP-driven de-identification as the primary detection path
- Video frame-by-frame OCR/redaction
- Automatic face redaction or voice transformation
- High-fidelity scanned-PDF layout reconstruction
- Full DICOM pixel-level burned-in text removal
- Full vendor-semantic understanding of every DICOM private tag or FCS keyword
- Multi-user enterprise approval chains or four-eyes decode approval
- External KMS/HSM integrations
- Plugin marketplace or unrestricted third-party code execution

## 3. Users and Usage Modes

### 3.1 Core user groups

- Medical data managers preparing datasets for sharing or AI workflows
- Research and data engineering teams processing structured/tabular and imaging data
- Compliance-sensitive operators who need review, traceability, and controlled re-identification
- Technical operators who need batch, scheduling, and automation

### 3.2 Usage modes

- Interactive review and governance
- Reusable scheduled pipelines
- Batch/headless execution
- Controlled decode and audit investigation

## 4. Detection, Mapping, and Trust Model

### 4.1 Detection strategy

v1 detection strategy is:

- rules
- dictionaries
- format-aware field/tag rules
- OCR-backed text detection

The architecture must be ready for future AI/NLP detectors, but v1 does not depend on them.

### 4.2 Reversibility model

The platform performs reversible de-identification by storing mappings in a protected vault. Re-identification is only permitted through explicit, auditable decode flows.

### 4.3 Deployment and trust model

- local-first
- offline-capable
- Windows-first
- pure Rust core
- sensitive assets remain local by default

## 5. Shared Workflow Model

All supported formats go through a shared top-level workflow. Format adapters change the extraction and rewrite depth, but not the core process.

### 5.1 Ingest

Inputs may include:

- single files
- folders
- recursive folder scans
- manifest/job files
- CLI arguments
- desktop import actions
- browser-triggered pipeline runs

Ingest performs:

- file/folder discovery
- file-kind classification
- adapter assignment
- job/run creation
- metadata registration
- fingerprint/checksum capture

### 5.2 Extract

Adapters normalize data into a shared intermediate representation (IR), which may include:

- filename/path identifiers
- metadata
- structured fields
- OCR text
- DICOM tags
- FCS metadata
- references to page/object/region scope

### 5.3 Detect

The detection pipeline emits PHI candidates with:

- type
- location/path
- confidence
- detector provenance
- supporting context

### 5.4 Review

Review is a first-class workflow stage. It supports:

- approve candidate
- reject false positive
- add missing candidate
- change PHI type
- adjust replacement strategy
- bulk apply decisions

### 5.5 Encode

Encoding performs governed replacement/tokenization and writes mapping state to the vault. All encode operations must be auditable.

### 5.6 Export

Exports may include:

- de-identified artifacts
- result summaries
- audit summaries
- scope-limited portable vault artifacts when policy allows

### 5.7 Decode

Decode is a high-risk operation. It requires:

- explicit unlock
- explicit scope selection
- explicit output target
- explicit reason/justification
- audit recording

### 5.8 Audit

Audit spans the entire system, not just decode:

- job/run creation
- detector activity
- review decisions
- encode actions
- vault operations
- decode operations
- exports
- failures and retries

## 6. Format Support and v1 Depth Grading

v1 supports six major format families with different support depth.

### 6.1 Depth levels

- **L3 — structured, strong reversible support**
- **L2 — content-aware, reversible with bounded fidelity**
- **L1 — container/filename/metadata-first support**
- **L0 — classify and track only**

### 6.2 Support matrix

| Format family | v1 level | v1 commitment | v1 non-commitment |
|---|---|---|---|
| DICOM | L3 | tag-level PHI handling, UID remap, filename/path handling, private-tag policy, burned-in suspicion flagging | full pixel-level text removal, full vendor-private semantics |
| CSV / Excel | L3 | schema-aware rules, field-level reversible mapping, batch consistency, decode | full macro/formula/business-logic rewriting |
| PDF / scanned records | L2 | text extraction, OCR, review, governed rewrite/export | perfect layout reconstruction, high-confidence handwriting support |
| FCS | L2/L3 metadata-first | TEXT/metadata identifier handling, encode/decode | full vendor-semantic interpretation |
| Images | L1 with OCR-assisted review | filename/path/metadata cleanup, OCR suspicion detection | full pixel-level redaction |
| Videos | L1 | filename/path/container metadata handling, sidecar handling | frame-level OCR/redaction, face or voice transformation |

### 6.3 DICOM

v1 DICOM scope includes:

- standard DICOM tag parsing
- common PHI tag handling
- UID family remapping
- filename and path sanitization
- private-tag policies: keep/remove/review_required
- suspicion flagging for burned-in annotation candidates

v1 DICOM does not claim complete pixel-level redaction.

### 6.4 CSV / Excel

v1 tabular scope includes:

- schema inference
- field-level PHI policies
- consistent tokenization across rows/files
- review/override
- decode
- batch summaries and partial-failure reporting

### 6.5 PDF / scanned records

v1 PDF scope includes:

- text-layer extraction when available
- OCR for scanned pages
- low-confidence review routing
- de-identified output generation
- summaries and review traces

### 6.6 Images

v1 image scope is conservative:

- filename/path handling
- metadata cleanup
- OCR-based suspicion/review where visible text exists

### 6.7 Videos

v1 video scope is governance-first, not content-redaction-first:

- filename/path handling
- container metadata cleanup
- sidecar file support
- audit and pipeline integration

### 6.8 FCS

v1 FCS support is metadata-first:

- TEXT segment parsing
- sample/patient identifier handling
- policy-driven encode/decode
- audit and result tracking

## 7. Reversibility Model by Format Family

Reversibility is not identical across every file family.

### 7.1 Structured formats

For structured formats such as DICOM tags, CSV/Excel cells, and FCS metadata, reversibility is true mapping-based reversible transformation.

### 7.2 Document/raster/media families

For scanned PDFs, images, and videos, v1 uses managed reversible recovery semantics:

- the de-identified artifact is a derived output
- original source association is managed
- decode restores from governed source + vault relationship rather than requiring impossible perfect inversion of every transformed pixel/frame output

## 8. Core Architecture

The system uses one Cargo workspace with focused crates/modules.

### 8.1 Workspace shape

```text
med-de-id/
├─ Cargo.toml
├─ crates/
│  ├─ mdid-domain/
│  ├─ mdid-policy/
│  ├─ mdid-detection/
│  ├─ mdid-vault/
│  ├─ mdid-adapters/
│  ├─ mdid-application/
│  ├─ mdid-runtime/
│  ├─ mdid-cli/
│  ├─ mdid-browser/
│  └─ mdid-desktop/
├─ fixtures/
├─ docs/
└─ tests/
```

### 8.2 `mdid-domain`

Defines core entities and state models:

- job
- pipeline definition
- pipeline run
- artifact
- PHI candidate
- review decision
- mapping record
- audit event
- decode request/result
- task/run states

### 8.3 `mdid-policy`

Defines:

- PHI taxonomy
- policy profiles
- field/tag rules
- replacement strategies
- thresholds and auto-approval rules

### 8.4 `mdid-detection`

Defines detector interfaces and built-in detector implementations:

- rules detector
- dictionary detector
- OCR-backed detector
- future AI/NLP detector boundary

### 8.5 `mdid-vault`

Owns:

- mapping storage model
- local encrypted vault store
- portable vault export/import model
- key wrapping/rewrap model
- decode lookup
- audit linkage for sensitive actions

### 8.6 `mdid-adapters`

Contains format adapters for:

- DICOM
- PDF
- tabular data
- images
- videos
- FCS

Each adapter is responsible for:

- ingest/parse
- extract to IR
- write back governed outputs

### 8.7 `mdid-application`

Defines shared application services/use cases such as:

- create job
- create or run pipeline
- ingest inputs
- run extract/detect
- submit review decisions
- encode/export
- decode
- query audit
- manage vault
- manage policies and schedules

### 8.8 `mdid-runtime`

Owns long-lived runtime behavior:

- pipeline execution
- scheduler
- run state persistence
- event emission
- review-task creation
- retry/resume behavior
- batch coordination

### 8.9 Surface crates

- `mdid-cli` — automation surface
- `mdid-browser` — browser workflow surface
- `mdid-desktop` — sensitive workstation surface

## 9. Security and Vault Design

### 9.1 Security principles

1. De-identified artifacts and re-identification capability must be separated.
2. Vault content is encrypted by default.
3. Decode is a high-risk, explicitly authorized operation.
4. Portable vault export must be scope-limited.
5. High-risk operations must be auditable.
6. The product governs data and operation risk; it does not claim to defend fully compromised hosts.

### 9.2 Threat model for v1

v1 is designed to reduce or control:

- local vault exposure after device loss
- accidental co-export of mappings with de-identified outputs
- untracked or overly broad decode operations
- sensitive values leaking into logs/temp files
- oversized portable vault exports

v1 does not claim to defeat:

- full host compromise
- live memory scraping by privileged attackers
- users intentionally exporting decrypted data unsafely

### 9.3 Vault forms

Two vault forms are supported:

1. **Local encrypted vault store** — primary working store
2. **Portable encrypted vault artifact** — scope-limited export/import container

### 9.4 Key model

The key model is passphrase-rooted with optional OS-protected local unlock support on Windows.

Conceptual key separation:

- `K_root` — derived from passphrase + salt
- `K_vault` — protects vault content
- `K_record` — per-record/per-object derived protection
- `K_export` — export-specific wrapping
- `K_audit_integrity` — audit-chain integrity material

### 9.5 Query restrictions

The system must not provide a general plain-text PHI search index. Querying should be based on:

- internal IDs
- tokens
- jobs/runs/artifacts
- bounded hashed/blind-index lookups when needed

### 9.6 Decode rules

Decode requires:

- vault unlock
- scope selection
- output target
- justification
- high-risk audit event

Default full-vault decode is not allowed.

### 9.7 Audit integrity

Audit storage must be tamper-evident using chained integrity links.

### 9.8 Sensitive-data hygiene

The system must avoid:

- plain PHI in normal logs
- uncontrolled temp outputs
- long-lived UI caches of revealed values
- accidental debug leaks

## 10. Tri-surface / Layered-responsibility Model

The product uses three formal surfaces with shared core capabilities and specialized responsibility.

### 10.1 Surface strategy

- **CLI** = automation surface
- **Browser tool** = workflow surface
- **Desktop app** = sensitive workstation surface

All three share the same Rust core, runtime, data model, policy logic, and security controls.

### 10.2 CLI responsibilities

CLI is optimized for:

- batch/headless runs
- automation and scripting
- CI/integration flows
- machine-readable results
- audit queries
- schedule/run control
- import/export of pipeline and policy artifacts

### 10.3 Browser tool responsibilities

Browser tool is optimized for:

- pipeline graph composition
- node/function configuration
- trigger/schedule configuration
- reusable workflow templates
- workflow monitoring
- run-level visibility

The browser tool may display summaries, but it is not the primary surface for heavy sensitive-data review or full vault maintenance.

### 10.4 Desktop responsibilities

Desktop is optimized for:

- file/dataset-centric ingest and inspection
- manual review/approve/override flows
- low-confidence handling
- vault operations
- decode/re-identification
- audit exploration
- failure triage and artifact investigation

### 10.5 Shared capability vs specialized UX

Core capability is shared; UX specialization is intentional. This means the runtime and application services expose one underlying capability model, but each surface emphasizes different workflows.

## 11. Pipeline, Node, and Schedule Model

### 11.1 Pipeline definition

A pipeline definition is a reusable workflow blueprint containing:

- node graph
- node config
- edges
- trigger/schedule bindings
- policy/profile bindings
- retry rules
- output rules
- review-gate rules

### 11.2 Pipeline run

A pipeline run is one concrete execution of a pipeline definition.

### 11.3 Review task

A review task is a governed human work item created by the runtime when a review gate is triggered.

### 11.4 Graph model

v1 uses a **controlled DAG**:

- directed
- acyclic
- supports branches/merges
- supports review pauses/gates
- supports retries via policy
- does not allow arbitrary graph loops

### 11.5 Node families

v1 nodes are grouped into these families:

- Trigger nodes
- Input/Ingest nodes
- Inspection/Extraction nodes
- Detection nodes
- Review Gate nodes
- Transform/Encode nodes
- Export/Delivery nodes
- Governance nodes
- Utility/Routing nodes

### 11.6 Trigger types

v1 trigger types:

- manual
- interval
- cron-like calendar schedule
- folder watch
- CLI/API invoke

### 11.7 Schedule policy

Each schedule must define:

- enabled/disabled
- timezone
- concurrency policy
- missed-run policy
- retry/backoff
- max active runs
- input binding
- output binding

Concurrency policies must include at least:

- `skip_if_running`
- `queue_if_running`
- `parallel_allowed`

Missed-run policies must include at least:

- `skip`
- `run_once_on_resume`
- `backfill_limited`

### 11.8 Review gates

Review gate nodes are first-class runtime pauses that generate human-governed work items. Trigger causes may include:

- low confidence
- policy-required confirmation
- suspicious burned-in DICOM text
- poor OCR quality
- sensitive output or decode approvals

### 11.9 Custom node/function strategy

v1 uses a three-layer extension model:

1. built-in nodes
2. parameterized nodes without user code
3. restricted custom expression/function nodes

v1 custom functions must remain constrained:

- pure data transformation or routing logic
- no unrestricted shell execution
- no unrestricted network access
- no bypass of audit
- no direct privileged vault/decode access

### 11.10 Runtime state model

Pipeline run states must include:

- `pending`
- `scheduled`
- `running`
- `waiting_for_review`
- `waiting_for_approval`
- `retrying`
- `completed`
- `partially_failed`
- `failed`
- `cancelled`

Review-task states must include:

- `open`
- `claimed`
- `resolved`
- `rejected`
- `expired`

## 12. Recommended v1 Technical Direction

### 12.1 Core and surfaces

- **Core/runtime/CLI/desktop:** Rust
- **Browser tool:** Rust WASM UI

### 12.2 Browser tool framework

The browser tool should use **Leptos** as the primary Rust WASM UI framework.

### 12.3 Desktop UI framework

The desktop app should use **egui** as the workstation UI framework.

### 12.4 Local runtime API

The runtime should expose a localhost-only API using **Axum** on `127.0.0.1` with:

- JSON request/response for control operations
- SSE for event streaming by default
- optional WebSocket support if needed for richer live coordination

### 12.5 OCR provider

OCR must be abstracted behind a provider interface. v1 should ship with a local **TesseractProvider** implementation.

### 12.6 Persistence

Persistence should use a **pure-Rust embedded store** with application-layer encryption. The preferred v1 direction is a `redb`-style embedded store model.

Persistence domains:

- vault store
- runtime store

### 12.7 Pipeline format

The canonical pipeline definition format is **versioned JSON**.

### 12.8 Runtime model

The runtime should be event-first, with standard events such as:

- `run_created`
- `run_started`
- `node_started`
- `node_completed`
- `review_task_created`
- `review_task_resolved`
- `run_waiting_for_review`
- `run_failed`
- `decode_requested`
- `decode_completed`
- `vault_exported`

## 13. v1 / v1.5 / v2 Roadmap

### 13.1 v1

v1 delivers:

- shared governed workflow
- vault and decode controls
- audit trail with tamper-evident chaining
- tri-surface model
- CSV/Excel deep support
- DICOM tag-level deep support
- PDF/OCR medium support
- image/video/FCS conservative governance-first support
- pipeline builder and scheduler with controlled DAG model

### 13.2 v1.5

v1.5 improves:

- detection quality/provenance
- OCR quality handling
- PDF review tooling
- DICOM private-tag policy templates
- image OCR review support
- key rotation/rewrap and export granularity
- parity and workflow polish across surfaces

### 13.3 v2

v2 may add:

- AI/NLP detector providers
- stronger image/pixel and media handling
- richer custom node/plugin model
- enterprise control features
- external KMS/HSM integrations

## 14. Slice Order for Implementation

### Slice 1 — platform skeleton

- workspace/crates
- domain models
- application/runtime skeleton
- basic CLI/browser/desktop entry points

### Slice 2 — vault + encode/decode minimum viable path

- local vault
- portable vault artifact
- audit for high-risk flows

### Slice 3 — CSV/Excel deep support

- tabular adapter
- rules/review/encode/decode path

### Slice 4 — DICOM tag-level support

- DICOM adapter
- tag handling
- UID remap
- review-required suspicion flagging

### Slice 5 — PDF/OCR support

- extraction/OCR
- review gates
- export/reporting

### Slice 6 — image/video/FCS conservative ingestion and governance

- metadata-first support
- pipeline/audit/policy integration

## 15. Non-functional Requirements

### 15.1 Reliability

The system must support:

- persisted job/run state
- partial-failure reporting
- retry/resume semantics
- consistent vault + runtime state transitions

### 15.2 Performance

v1 must provide:

- predictable progress reporting
- non-frozen UI during long-running local jobs
- observable stage-level progress for OCR/detect/encode flows

v1 does not aim for distributed-scale throughput optimization.

### 15.3 Auditability

The system must support answering:

- what was processed
- which detector/rule acted
- what was approved/overridden
- who performed decode or export
- why the action occurred
- where outputs were written

### 15.4 Security hygiene

v1 must avoid low-level mistakes such as plain PHI in logs or uncontrolled temp output.

### 15.5 Testability

Each layer must be testable independently, and fixtures are part of the design, not an afterthought.

## 16. Acceptance Criteria

`med-de-id v1` is acceptable only if all of the following are true:

- the shared workflow is fully runnable end-to-end
- vault and decode controls work as designed
- audit is queryable and tamper-evident
- CSV/Excel deep support is functional
- DICOM tag-level deep support is functional
- PDF text/OCR medium support is functional
- image/video/FCS are integrated honestly with conservative scope
- Windows-first operation is validated as the primary platform
- browser tool can define and schedule pipelines
- desktop can perform governed review/vault/decode/audit workflows
- CLI can automate and inspect the system headlessly
- no critical flow relies on cloud connectivity

## 17. End-to-end Validation Scenarios

The spec requires at least these E2E validation flows:

1. CSV batch reversible flow
2. DICOM tag-level reversible flow
3. PDF OCR review flow
4. High-risk decode governance flow
5. Pipeline definition + scheduled/local run + review task handoff flow
6. Cross-surface parity checks for core runtime outcomes

## 18. Final Design Statement

`med-de-id v1` is a Windows-first, local-first, pure-Rust-core medical de-identification platform with:

- reversible governed mapping
- auditable decode and export controls
- broad format-family coverage with explicit depth grading
- a tri-surface product model
- a browser-based workflow tool
- a desktop sensitive-data workstation
- a CLI automation surface
- a controlled DAG pipeline engine with review gates and local scheduling

This design deliberately favors truthful scope, strong workflow integrity, and governed local processing over inflated claims of universal automatic de-identification.
