# CLI completion-impact batch

## Goal
Close bounded CLI/product-core completion blockers without claiming Browser/Desktop/OCR/visual redaction or media-byte rewrite completion.

## Scope
- Add conservative media metadata-only export evidence to the CLI review-media flow.
- Add a PHI-safe portable transfer summary CLI alias for encrypted vault artifacts.
- Truth-sync README after tests pass.

## TDD evidence
- RED: `cargo test -p mdid-cli cli_review_media_can_export_metadata_only_rewrite_evidence -- --nocapture` failed on unknown `--export-report`.
- RED: `cargo test -p mdid-cli parses_portable_transfer_summary_command -- --nocapture` failed because `CliCommand::PortableTransferSummary` did not exist.
- GREEN: targeted tests pass after implementation.

## Non-goals
- No media-byte rewrite/export.
- No OCR completion, visual redaction, PDF rewrite/export, Browser/Desktop/controller semantics, or network workflow changes.
