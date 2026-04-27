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
| PDF / scanned records | L2 | text extraction, OCR, review, governed rewrite/export |
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

This repository currently contains the Slice 1 workspace foundation, the Slice 2 vault MVP, and the first Slice 3 tabular workflow and adapter work.

Implemented so far:

- Shared domain models for pipeline, review, vault mapping, decode requests, audit events, and tabular workflow state
- An encrypted `mdid-vault` crate with local file-backed storage, explicit decode-by-record-id, audit recording, portable subset export, and repeated-value token reuse
- An implemented `mdid-adapters` crate with shared tabular extraction for CSV/XLSX inputs, schema inference, field-level PHI candidate policies, and blank-cell handling parity
- Tabular application orchestration that composes the adapters with vault-backed reversible encoding and honest batch summaries
- Initial `mdid-runtime`, `mdid-cli`, `mdid-browser`, and `mdid-desktop` scaffolding from the foundation slice

Planned next from the design:

- Additional policy and detection crates
- Deeper application orchestration and surface behavior beyond the current scaffolds

Available docs:

- Design spec: `docs/superpowers/specs/2026-04-25-med-de-id-design.md`
- Initial implementation plan: `docs/superpowers/plans/2026-04-25-med-de-id-foundation-implementation-plan.md`
- Slice 2 vault/decode MVP plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-2-vault-encode-decode-mvp.md`
- Slice 3 tabular deep-support plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-3-tabular-deep-support.md`

## Roadmap

- **v1**: governed workflow core, vault/decode controls, audit trail, tri-surface skeleton, deep CSV/Excel + DICOM tag-level support, medium PDF/OCR support, conservative image/video/FCS support
- **v1.5**: detection quality/provenance upgrades, PDF/DICOM policy depth, parity and workflow polish
- **v2**: AI/NLP detectors, stronger media handling, richer custom node/plugin model, enterprise controls

## Repo conventions

- Planning and design docs live under `docs/superpowers/`
- Implementation is expected to follow TDD and small verified slices
- The browser tool is local-first and served on `127.0.0.1`, not a SaaS deployment

## License

Workspace metadata is currently marked `UNLICENSED`.
