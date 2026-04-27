# med-de-id

Windows-first, local-first medical de-identification platform with a pure Rust core.

## What it is

`med-de-id` is a governed workflow system for reversible medical data de-identification. It is designed for local/offline use, keeps sensitive assets on-device by default, and supports controlled decode/re-identification with audit trails.

The product has three formal surfaces:

1. **CLI** — automation, batch, integration, headless execution
2. **Browser tool** — localhost pipeline/orchestration workbench for workflow composition and scheduling
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
| CSV / Excel | L3 | schema-aware reversible mapping and batch consistency |
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

This repository currently contains the Slice 1 workspace foundation, the Slice 2 vault MVP, the first Slice 3 tabular workflow and adapter work, the bounded Slice 5/6 PDF support foundation, and bounded runtime HTTP entries for DICOM de-identification, tabular CSV de-identification, vault decode, bounded vault audit browsing, bounded portable subset export, bounded portable artifact inspection, and bounded portable artifact import into a local vault.

Implemented so far:

- Shared domain models for pipeline, review, vault mapping, decode requests, audit events, and tabular workflow state
- An encrypted `mdid-vault` crate with local file-backed storage, explicit decode-by-record-id, audit recording, portable subset export, bounded portable artifact import, deterministic duplicate/normalization handling via the shared import contract, and repeated-value token reuse
- An implemented `mdid-adapters` crate with shared tabular extraction for CSV/XLSX inputs, schema inference, field-level PHI candidate policies, and blank-cell handling parity
- Tabular application orchestration that composes the adapters with vault-backed reversible encoding and honest batch summaries
- Bounded PDF support for text-layer extraction, OCR-needed suspicion routing, mixed multi-page summary/reporting, and invalid-PDF rejection as parse failure
- Current PDF support does not yet perform full OCR, visual redaction, handwriting handling, or final PDF rewrite/export
- `mdid-runtime` now exposes a bounded local HTTP DICOM de-identification entry that accepts local/base64-transported DICOM bytes, applies the existing private-tag policy service logic, returns rewritten DICOM bytes plus a review summary/review queue, and honestly rejects invalid DICOM payloads
- `mdid-runtime` also exposes a bounded local HTTP tabular de-identification entry that currently stays scoped to CSV request bodies, accepts CSV text plus explicit field policies, returns rewritten CSV plus a summary and review queue, and does not yet imply XLSX uploads or broader tabular import/export APIs
- `mdid-runtime` also exposes a bounded local HTTP vault decode entry that unlocks a local vault with an explicit passphrase, decodes only the requested record scope, returns decoded values plus the resulting audit event, and honestly rejects wrong passphrases, unknown records, invalid decode requests, and unusable vault targets
- `mdid-runtime` also exposes a bounded local HTTP vault audit browsing entry that unlocks a local vault with an explicit passphrase, returns persisted audit events in reverse chronological order with bounded filtering, supports filtering by event kind and actor, and remains read-only
- `mdid-runtime` also exposes a bounded local HTTP portable export entry that unlocks a local vault with an explicit passphrase, exports only the requested bounded record subset into an encrypted portable artifact, records the resulting export audit event, and remains scoped to local export creation rather than import or transfer workflows
- `mdid-runtime` also exposes a bounded local HTTP portable artifact inspection entry that locally unlocks an encrypted portable artifact with an explicit portable passphrase and returns a bounded preview of persisted record fields from the encrypted artifact contents, including sensitive persisted values already stored in the artifact such as tokens and original values
- `mdid-runtime` also exposes a bounded local HTTP portable artifact import entry that unlocks a local vault with an explicit vault passphrase, imports an encrypted portable artifact into that local vault, skips duplicate record ids and existing semantic duplicates while deterministically normalizing shared-value token reuse through the shared import contract, records the resulting import audit event, and returns bounded imported/duplicate counts rather than artifact contents or generalized transfer state
- `mdid-cli`, `mdid-browser`, and `mdid-desktop` remain early surface scaffolds

The current runtime HTTP slice is intentionally narrow: it is still bounded to local request bodies for DICOM, CSV/tabular, vault decode, bounded audit browsing, bounded portable export creation, bounded portable artifact inspection, and bounded portable artifact import into a local vault. The import route is limited to local vault persistence of encrypted portable artifacts with bounded imported/duplicate counts plus an audit event; it does not yet provide generalized transfer orchestration, auth/session handling, remote handoff workflows, full audit search, audit mutation workflows, XLSX upload support, generalized vault browsing, or generalized decode.

Planned next from the design:

- Additional policy and detection crates
- Deeper application orchestration and surface behavior beyond the current scaffolds

Available docs:

- Design spec: `docs/superpowers/specs/2026-04-25-med-de-id-design.md`
- Initial implementation plan: `docs/superpowers/plans/2026-04-25-med-de-id-foundation-implementation-plan.md`
- Slice 2 vault/decode MVP plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-2-vault-encode-decode-mvp.md`
- Slice 3 tabular deep-support plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-3-tabular-deep-support.md`

## Roadmap

- **v1**: governed workflow core, vault/decode controls, audit trail, tri-surface skeleton, deep CSV/Excel + DICOM tag-level support, bounded PDF/scanned-record foundation, conservative image/video/FCS support
- **v1.5**: detection quality/provenance upgrades, PDF/DICOM policy depth, parity and workflow polish
- **v2**: AI/NLP detectors, stronger media handling, richer custom node/plugin model, enterprise controls

## Repo conventions

- Planning and design docs live under `docs/superpowers/`
- Implementation is expected to follow TDD and small verified slices
- The browser tool is local-first and served on `127.0.0.1`, not a SaaS deployment

## License

Workspace metadata is currently marked `UNLICENSED`.
