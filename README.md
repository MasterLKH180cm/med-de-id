# med-de-id

Windows-first, local-first medical de-identification system with a pure Rust core.

## What it is

`med-de-id` is a governed workflow system for reversible medical data de-identification. It is designed for local/offline use, keeps sensitive assets on-device by default, and supports controlled decode/re-identification with audit trails.

The product has three formal surfaces:

1. **CLI** — automation, batch, integration, headless execution
2. **Browser tool** — local-first browser surface, currently limited to bounded tabular de-identification and PDF review pages served on localhost
3. **Desktop app** — sensitive workstation for review, vault operations, decode flows, and audit investigation

## Core workflow

```text
ingest -> extract -> detect -> review -> encode -> export -> decode -> audit
```

## Design principles

- **Pure Rust core**
- **Windows-first**
- **Local-first / offline-capable**
- **Reversible mapping stored in a protected vault**
- **High-risk decode is explicit and auditable**
- **Broad format-family coverage with honest depth grading**
- **Tri-surface product model with layered responsibility**

## Planned format support

| Format family | v1 depth | Notes |
|---|---|---|
| DICOM | L3 | tag-level handling, UID remap, private-tag policy, burned-in suspicion flagging |
| CSV | L3 | schema-aware reversible mapping and batch consistency |
| Excel (XLSX, bounded) | L2 | schema-aware reversible mapping on only the first non-empty worksheet; rewritten output preserves that bounded single-sheet flow and does not offer caller-controlled sheet selection |
| PDF / scanned records | L1/L2 foundation | text-layer extraction, OCR-needed suspicion routing, mixed multi-page summary/reporting, and invalid-PDF rejection as parse failure; no full OCR, visual redaction, handwriting handling, or final PDF rewrite/export yet |
| FCS | L2/L3 metadata-first | TEXT/metadata identifier handling |
| Images | L1 | filename/path/metadata cleanup, OCR-assisted suspicion |
| Videos | L1 | filename/path/container metadata and sidecar handling |

## Architecture overview

```text
shared Rust workspace today
├─ mdid-domain
├─ mdid-vault
├─ mdid-adapters
├─ mdid-application
├─ mdid-runtime
├─ mdid-cli
├─ mdid-browser
└─ mdid-desktop
```

Planned follow-on core crates from the design, not yet implemented in this repository:

- `mdid-policy`
- `mdid-detection`

## Current repository status

Completion snapshot, based only on landed repository features and verification state:

| Area | Completion | Status |
|---|---:|---|
| CLI | 42% | Early automation surface with local de-identification, vault/decode, audit, and import/export entry points; unrelated scope-drift legacy commands are not counted as product completion. |
| Browser/web | 34% | Bounded localhost tabular de-identification page plus bounded PDF review mode backed by local runtime routes, with bounded CSV/XLSX/PDF file import/export helper controls; not a broader browser governance workspace. |
| Desktop app | 35% | Bounded sensitive-workstation foundation prepares CSV, XLSX, PDF review, DICOM, bounded vault decode/audit, and portable artifact export/inspect/import request envelopes for existing localhost runtime routes, can apply bounded CSV/XLSX/PDF/DICOM file import/export helpers, submit prepared non-vault and portable helper envelopes to a localhost runtime, and render response panes with honest disclosures; deeper desktop vault browsing, decode workflow execution UX, audit investigation workflow, portable transfer execution UX, OCR, visual redaction, PDF rewrite/export, full DICOM review workflow, and full review workflows remain unimplemented. |
| Overall | 49% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review/PDF review/DICOM/vault decode/audit/portable export/import entries, browser tabular/PDF review surface with bounded CSV/XLSX/PDF import/export helpers, desktop request-preparation/localhost-submit/response workbench foundation with bounded CSV/XLSX/PDF/DICOM file import/export helpers and bounded desktop vault/portable request-preparation helpers, and local CLI foundations are present; major workflow depth and surface parity remain missing; unrelated scope-drift legacy CLI commands are not counted as core product progress. |

Missing items include deeper policy/detection crates, full review/governance workflows, richer browser UX including deeper upload/download UX beyond bounded CSV/XLSX/PDF import/export helpers, deeper desktop vault browsing, decode workflow execution UX, audit investigation workflow, portable transfer UX, desktop PDF flow beyond request preparation and bounded review/export helper naming, desktop DICOM flow beyond bounded request/response/import/export helper support, broader import/export and upload flows, OCR, visual redaction, handwriting handling, full PDF rewrite/export, FCS semantic parsing, media rewrite/export, generalized spreadsheet handling, auth/session handling where needed, removal or isolation of scope-drift legacy CLI surfaces from product-facing documentation and roadmap claims, and production packaging/hardening.

This repository currently contains the Slice 1 workspace foundation, the Slice 2 vault MVP, the first Slice 3 tabular workflow and adapter work, the bounded Slice 5/6 PDF support foundation, and bounded runtime HTTP entries for DICOM de-identification, tabular CSV/XLSX de-identification, PDF review, conservative media metadata review, vault decode, bounded vault audit browsing, bounded portable subset export, bounded portable artifact inspection, and bounded portable artifact import into a local vault.

Implemented so far:

- Shared domain models for pipeline, review, vault mapping, decode requests, audit events, and tabular workflow state
- Conservative media/FCS domain workflow models, bounded adapter foundation, and application-layer review routing now distinguish image/video/FCS metadata-only status, OCR-or-visual-review-required status, unsupported payloads, review-required metadata candidates, honest summary counts, and redacted candidate/reference/output debug; this does not implement OCR, visual redaction, FCS semantic parsing, rewrite/export, or browser/desktop flows
- An encrypted `mdid-vault` crate with local file-backed storage, explicit decode-by-record-id, audit recording, portable subset export, bounded portable artifact import, deterministic duplicate/normalization handling via the shared import contract, and repeated-value token reuse
- An implemented `mdid-adapters` crate with shared tabular extraction for CSV/XLSX inputs, schema inference, field-level PHI candidate policies, and blank-cell handling parity
- CSV/tabular import hardening strips a leading UTF-8 BOM from the first header before policy matching, so BOM-prefixed CSV exports still match explicit field policies; this is a narrow adapter normalization and does not broaden upload/import workflows
- Tabular application behavior that composes the adapters with vault-backed reversible encoding and honest batch summaries
- Bounded PDF support for text-layer extraction, OCR-needed suspicion routing, mixed multi-page summary/reporting, and invalid-PDF rejection as parse failure
- Current PDF support does not yet perform full OCR, visual redaction, handwriting handling, or final PDF rewrite/export
- `mdid-runtime` now exposes a bounded local HTTP DICOM de-identification entry that accepts local/base64-transported DICOM bytes, applies the existing private-tag policy service logic, returns rewritten DICOM bytes plus a review summary/review queue, and honestly rejects invalid DICOM payloads
- `mdid-runtime` also exposes bounded local HTTP tabular de-identification entries: one accepts CSV text plus explicit field policies and returns rewritten CSV plus a summary and review queue; another accepts base64-transported XLSX workbook bytes plus explicit field policies and returns rewritten workbook bytes plus a summary and review queue, but only extracts and rewrites the first non-empty worksheet and does not offer caller-controlled sheet selection. These entries still do not imply multipart upload flows, generalized spreadsheet browsing/import/export APIs, workbook-wide fidelity guarantees, or any auth/session handling
- `mdid-runtime` also exposes a bounded local HTTP `/pdf/deidentify` review entry that accepts base64 PDF bytes plus a source name, delegates to the existing PDF application service, reports the existing text-layer/OCR-required summary, page statuses, and review queue, and returns `rewritten_pdf_bytes_base64: null`; it does not perform OCR, handwriting handling, visual redaction, PDF rewrite/export, desktop PDF flow, auth/session handling, generalized upload workflows, or broader workflow behavior
- `mdid-runtime` exposes a bounded local HTTP conservative media review entry accepting image/video/FCS metadata JSON, routes through the application review service, returns a summary/review queue and `rewritten_media_bytes_base64: null`; it explicitly does not implement OCR, visual redaction, FCS semantic parsing, media rewrite/export, multipart upload, browser/desktop flows, auth/session, or generalized media workflow behavior
- `mdid-runtime` also exposes a bounded local HTTP vault decode entry that unlocks a local vault with an explicit passphrase, decodes only the requested record scope, returns decoded values plus the resulting audit event, and honestly rejects wrong passphrases, unknown records, invalid decode requests, and unusable vault targets
- `mdid-runtime` also exposes a bounded local HTTP vault audit browsing entry that unlocks a local vault with an explicit passphrase, returns persisted audit events in reverse chronological order with bounded filtering, supports filtering by event kind and actor, and remains read-only
- `mdid-runtime` also exposes a bounded local HTTP portable export entry that unlocks a local vault with an explicit passphrase, exports only the requested bounded record subset into an encrypted portable artifact, records the resulting export audit event, and remains scoped to local export creation rather than import or transfer workflows
- `mdid-runtime` also exposes a bounded local HTTP portable artifact inspection entry that locally unlocks an encrypted portable artifact with an explicit portable passphrase and returns a bounded preview of persisted record fields from the encrypted artifact contents, including sensitive persisted values already stored in the artifact such as tokens and original values
- `mdid-runtime` also exposes a bounded local HTTP portable artifact import entry that unlocks a local vault with an explicit vault passphrase, imports an encrypted portable artifact into that local vault, skips duplicate record ids and existing semantic duplicates while deterministically normalizing shared-value token reuse through the shared import contract, records the resulting import audit event, and returns bounded imported/duplicate counts rather than artifact contents or generalized transfer state
- `mdid-browser` is no longer only a scaffold: it now provides a local-first browser page for a bounded tabular de-identification flow that submits CSV text or base64-transported XLSX workbook bytes plus explicit field policies to the local `mdid-runtime` on localhost and renders the bounded rewritten result, summary, and review queue that come back; it also provides a bounded PDF review mode backed by `/pdf/deidentify` that submits base64 PDF bytes plus a source name and renders the review-only summary, page statuses, and review queue, plus bounded CSV/XLSX/PDF file import/export helper controls that do not broaden the localhost runtime contracts
- `mdid-browser` still does not provide OCR, visual redaction, handwriting handling, PDF rewrite/export, richer browser upload/download UX depth, desktop PDF flow, auth/session handling, generalized workflow behavior, a generalized workflow builder, or any broader browser governance workspace
- `mdid-desktop` now renders a bounded sensitive-workstation foundation for preparing local runtime CSV, XLSX, PDF review, DICOM, vault decode/audit, and portable artifact export/inspect/import requests with endpoint previews, validation status, mode-specific disclosures, bounded CSV/XLSX/PDF/DICOM file import/export helpers, localhost runtime submission, and local runtime-shaped summary, review queue, rewritten-output/review-notice, and error panes. It still does not implement deeper desktop vault browsing, decode workflow execution UX, audit investigation workflow, portable transfer execution UX, OCR, visual redaction, PDF rewrite/export, desktop DICOM flow beyond bounded request/response/import/export helper support, full DICOM review workflow, or full review workflows
- `mdid-cli` remains an early de-identification automation surface for local workflows such as vault/decode, audit, and import/export entry points. Unrelated scope-drift legacy commands are not part of the de-identification roadmap and are not counted toward completion; future stop-loss cleanup should remove or isolate them rather than expand them.

The current runtime HTTP slice is intentionally narrow: it is still bounded to local request bodies for DICOM, CSV/tabular, base64-transported XLSX workbook bytes, base64-transported PDF review, conservative media metadata review, vault decode, bounded audit browsing, bounded portable export creation, bounded portable artifact inspection, and bounded portable artifact import into a local vault. The XLSX route is limited to returning rewritten workbook bytes plus a summary/review queue for only the first non-empty worksheet that it extracts and rewrites; it does not let callers choose a worksheet and should not be read as workbook-wide Excel handling. The PDF route is limited to accepting base64 PDF bytes plus a source name and returning existing text-layer/OCR-required summary, page statuses, review queue, and `rewritten_pdf_bytes_base64: null`; the browser PDF mode is only a bounded local client for that route and does not perform OCR, handwriting handling, visual redaction, PDF rewrite/export, browser upload UX, desktop PDF flow, auth/session handling, generalized upload workflows, or broader workflow behavior. The conservative media route is limited to image/video/FCS metadata JSON review through the application service and returns summary/review queue data with `rewritten_media_bytes_base64: null`; it does not perform OCR, visual redaction, FCS semantic parsing, media rewrite/export, multipart upload, browser/desktop flows, auth/session, or generalized media workflow behavior. The import route is limited to local vault persistence of encrypted portable artifacts with bounded imported/duplicate counts plus an audit event. These routes do not add multipart upload handling, generalized spreadsheet browsing/import/export APIs, auth/session handling, or any broader multi-step transfer flow.

Planned next from the design:

- Additional policy and detection crates
- Deeper application behavior and de-identification surface behavior beyond the current scaffolds
- Stop-loss cleanup to remove or isolate scope-drift legacy CLI surfaces from product-facing documentation and roadmap claims

Available docs:

- Design spec: `docs/superpowers/specs/2026-04-25-med-de-id-design.md`
- Initial implementation plan: `docs/superpowers/plans/2026-04-25-med-de-id-foundation-implementation-plan.md`
- Slice 2 vault/decode MVP plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-2-vault-encode-decode-mvp.md`
- Slice 3 tabular deep-support plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-3-tabular-deep-support.md`

## Roadmap

- **v1**: governed workflow core, vault/decode controls, audit trail, tri-surface skeleton, deep CSV support, bounded XLSX runtime support, DICOM tag-level support, bounded PDF/scanned-record foundation, conservative image/video/FCS support
- **v1.5**: detection quality/provenance upgrades, PDF/DICOM policy depth, parity and workflow polish
- **v2**: AI/NLP detectors, stronger media handling, richer custom node/plugin model, enterprise controls

## Repo conventions

- Planning and design docs live under `docs/superpowers/`
- Implementation is expected to follow TDD and small verified slices
- The browser tool is local-first and served on `127.0.0.1`, not a SaaS deployment

## License

Workspace metadata is currently marked `UNLICENSED`.
