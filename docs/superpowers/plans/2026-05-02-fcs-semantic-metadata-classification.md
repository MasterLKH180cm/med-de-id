# FCS Semantic Metadata Classification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded FCS semantic metadata classifier so FCS TEXT-key metadata is categorized as field-level PHI candidates rather than generic metadata-only identifiers.

**Architecture:** Extend the existing conservative media adapter with FCS-specific semantic key classification for known TEXT segment keys while preserving the existing conservative review contract. The slice remains byte-rewrite neutral: it classifies field-level PHI for review/export safety, but does not claim FCS payload rewrite.

**Tech Stack:** Rust workspace, `mdid-adapters`, `mdid-domain`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-adapters/src/conservative_media.rs` — add FCS semantic key classification helper and use it only for `ConservativeMediaFormat::Fcs` candidates.
- Modify: `crates/mdid-adapters/tests/conservative_media_adapter.rs` — add TDD coverage for FCS patient/sample/operator/date keys and conservative fallback.
- Modify: `README.md` — truth-sync FCS status after repository-visible verification.

### Task 1: FCS semantic PHI classification in conservative media adapter

**Files:**
- Modify: `crates/mdid-adapters/tests/conservative_media_adapter.rs`
- Modify: `crates/mdid-adapters/src/conservative_media.rs`

- [ ] **Step 1: Write the failing FCS semantic classification test**

Add this test to `crates/mdid-adapters/tests/conservative_media_adapter.rs`:

```rust
#[test]
fn fcs_metadata_uses_semantic_phi_types_for_known_text_keys() {
    let input = ConservativeMediaInput {
        artifact_label: "flow/panel.fcs".to_string(),
        format: ConservativeMediaFormat::Fcs,
        metadata: vec![
            metadata_entry("$FIL", "Jane-Doe-panel.fcs"),
            metadata_entry("$SMNO", "MRN-12345"),
            metadata_entry("$SRC", "Bone Marrow aspirate"),
            metadata_entry("$OP", "Dr. Alice Example"),
            metadata_entry("$DATE", "2026-04-23"),
            metadata_entry("CUSTOM_NOTE", "Research subject Jane Example"),
        ],
        requires_visual_review: false,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    let phi_types = output
        .candidates
        .iter()
        .map(|candidate| (candidate.field_ref.metadata_key.as_str(), candidate.phi_type.as_str()))
        .collect::<Vec<_>>();

    assert_eq!(
        phi_types,
        vec![
            ("$FIL", "fcs_filename_identifier"),
            ("$SMNO", "fcs_sample_identifier"),
            ("$SRC", "fcs_source_identifier"),
            ("$OP", "fcs_operator_identifier"),
            ("$DATE", "fcs_collection_date"),
            ("CUSTOM_NOTE", "metadata_identifier"),
        ]
    );
    assert!(output
        .candidates
        .iter()
        .all(|candidate| candidate.status == ConservativeMediaScanStatus::MetadataOnly));
    assert_eq!(output.summary.review_required_candidates, 6);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter fcs_metadata_uses_semantic_phi_types_for_known_text_keys -- --nocapture`

Expected: FAIL because FCS candidates still use `metadata_identifier` for every key.

- [ ] **Step 3: Implement minimal FCS key classifier**

In `crates/mdid-adapters/src/conservative_media.rs`, add a helper:

```rust
fn classify_conservative_media_phi_type(
    format: ConservativeMediaFormat,
    metadata_key: &str,
) -> &'static str {
    if format != ConservativeMediaFormat::Fcs {
        return METADATA_IDENTIFIER_PHI_TYPE;
    }

    match metadata_key.trim().to_ascii_uppercase().as_str() {
        "$FIL" | "FILENAME" | "FILE" => "fcs_filename_identifier",
        "$SMNO" | "SMNO" | "SAMPLE_ID" | "SAMPLEID" | "SPECIMEN_ID" => {
            "fcs_sample_identifier"
        }
        "$SRC" | "SRC" | "SOURCE" | "SPECIMEN_SOURCE" => "fcs_source_identifier",
        "$OP" | "OP" | "OPERATOR" | "CREATOR" => "fcs_operator_identifier",
        "$DATE" | "DATE" | "COLLECTION_DATE" | "ACQUISITION_DATE" => "fcs_collection_date",
        _ => METADATA_IDENTIFIER_PHI_TYPE,
    }
}
```

Then change candidate construction from:

```rust
phi_type: METADATA_IDENTIFIER_PHI_TYPE.to_string(),
```

to:

```rust
phi_type: classify_conservative_media_phi_type(input.format, &entry.key).to_string(),
```

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter fcs_metadata_uses_semantic_phi_types_for_known_text_keys -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run adapter regression tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-adapters/src/conservative_media.rs crates/mdid-adapters/tests/conservative_media_adapter.rs docs/superpowers/plans/2026-05-02-fcs-semantic-metadata-classification.md
git commit -m "feat(fcs): classify semantic metadata candidates"
```

### Task 2: Truth-sync README for FCS semantic classification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update FCS status wording**

Change the planned support FCS row to state that FCS now has field-level TEXT-key semantic PHI classification for known keys (`$FIL`, `$SMNO`, `$SRC`, `$OP`, `$DATE`) with conservative metadata fallback, while still lacking FCS byte rewrite/export.

- [ ] **Step 2: Add verification evidence paragraph**

Add a paragraph under the current repository status stating the exact tests run and non-goals.

- [ ] **Step 3: Run documentation diff check and targeted tests**

Run: `git diff --check && source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs(readme): truth-sync fcs semantic classification"
```

## Self-Review

- Spec coverage: Covers priority item 6 (FCS semantic parsing) at the semantic metadata classification layer and keeps byte rewrite/export as an explicit non-goal.
- Placeholder scan: No TBD/TODO/implement-later placeholders.
- Type consistency: Uses existing `ConservativeMediaFormat`, `ConservativeMediaScanStatus`, and `ConservativeMediaCandidate.phi_type` fields.
