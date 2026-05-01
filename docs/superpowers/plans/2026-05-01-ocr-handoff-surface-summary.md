# OCR handoff surface summary plan

Date: 2026-05-01
Branch: feat/privacy-filter-cli-spike-cron-2117
Scope: Browser/Desktop PHI-safe summaries for existing OCR handoff JSON reports only.

## Tasks

1. Add RED tests in `mdid-browser` for a Browser download helper that summarizes OCR handoff JSON with only safe fields and no `normalized_text` or synthetic PHI sentinels.
2. Add RED tests in `mdid-desktop` for a Desktop save/helper payload with matching safe summary semantics and PHI leak assertions.
3. Implement minimal serde_json allowlist sanitizers; no OCR execution, Privacy Filter execution, UI orchestration, visual redaction, pixel redaction, PDF rewrite/export, or unrelated orchestration semantics.
4. Update README completion snapshot/evidence truthfully: Browser/Web 98%, Desktop app 98%, CLI 95%, Overall 96%, target threshold 99%.
5. Run targeted `ocr_handoff` tests, format/check diffs, and commit.
