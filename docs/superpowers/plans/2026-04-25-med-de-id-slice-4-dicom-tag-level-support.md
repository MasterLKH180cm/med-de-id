# med-de-id Slice 4 DICOM Tag-Level Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first DICOM tag-level reversible flow with common PHI tag handling, UID remap, private-tag policy decisions, filename/path sanitization, and honest burned-in-annotation suspicion routing.

**Architecture:** This slice extends `mdid-domain` with DICOM-specific workflow vocabulary, adds a focused DICOM adapter inside `mdid-adapters`, and composes it with `mdid-vault` through `mdid-application`. The adapter owns DICOM parsing and governed write-back; the application layer owns reversible mapping, review routing, summary accounting, and keeps PHI out of `Debug` output.

**Tech Stack:** Rust workspace, Cargo, Serde, UUID, Chrono, thiserror, tempfile, `dicom-core`, `dicom-object`.

---

## Scope note

This plan covers **Slice 4 — DICOM tag-level support** only. It does not implement pixel-level redaction, full private-tag semantics, or PDF/image/video/FCS support. To keep the scope honest and shippable, this slice lands in five narrow tasks:

1. DICOM workflow/domain vocabulary
2. DICOM adapter extraction for common PHI tags and private-tag policy classification
3. DICOM write-back with UID remap and filename/path sanitization
4. Application orchestration + vault-backed reversible mapping + review routing
5. Docs, workspace verification, and milestone truth-sync

## File structure

**Create:**
- `crates/mdid-domain/tests/dicom_workflow_models.rs`
- `crates/mdid-adapters/src/dicom.rs`
- `crates/mdid-adapters/tests/dicom_adapter.rs`
- `crates/mdid-application/tests/dicom_deidentification.rs`

**Modify:**
- `Cargo.toml`
- `README.md`
- `crates/mdid-domain/src/lib.rs`
- `crates/mdid-adapters/Cargo.toml`
- `crates/mdid-adapters/src/lib.rs`
- `crates/mdid-application/Cargo.toml`
- `crates/mdid-application/src/lib.rs`

---

### Task 1: Add DICOM workflow vocabulary to `mdid-domain`

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/dicom_workflow_models.rs`

- [ ] **Step 1: Write the failing domain tests**

Create `crates/mdid-domain/tests/dicom_workflow_models.rs`:

```rust
use mdid_domain::{
    BurnedInAnnotationStatus, DicomDeidentificationSummary, DicomPhiCandidate,
    DicomPrivateTagPolicy, DicomTagRef, ReviewDecision,
};

#[test]
fn dicom_tag_ref_builds_a_stable_field_path_and_detects_private_groups() {
    let patient_name = DicomTagRef::new(0x0010, 0x0010, "PatientName".into());
    let private_creator = DicomTagRef::new(0x0011, 0x0010, "PrivateCreator".into());

    assert_eq!(patient_name.field_path(), "dicom/0010,0010/PatientName");
    assert!(!patient_name.is_private());
    assert!(private_creator.is_private());
}

#[test]
fn dicom_policy_wire_values_are_stable() {
    assert_eq!(serde_json::to_string(&DicomPrivateTagPolicy::ReviewRequired).unwrap(), "\"review_required\"");
    assert_eq!(serde_json::to_string(&BurnedInAnnotationStatus::Suspicious).unwrap(), "\"suspicious\"");
}

#[test]
fn dicom_phi_candidate_debug_redacts_phi() {
    let candidate = DicomPhiCandidate {
        tag: DicomTagRef::new(0x0010, 0x0010, "PatientName".into()),
        phi_type: "patient_name".into(),
        value: "Alice Smith".into(),
        decision: ReviewDecision::Approved,
    };

    let debug = format!("{candidate:?}");
    assert!(debug.contains("DicomPhiCandidate"));
    assert!(!debug.contains("Alice Smith"));
}

#[test]
fn dicom_summary_requires_review_for_review_items_or_burned_in_suspicion() {
    let review_summary = DicomDeidentificationSummary {
        review_required_tags: 1,
        ..DicomDeidentificationSummary::default()
    };
    let suspicious_summary = DicomDeidentificationSummary {
        burned_in_suspicions: 1,
        ..DicomDeidentificationSummary::default()
    };

    assert!(review_summary.requires_review());
    assert!(suspicious_summary.requires_review());
    assert!(!DicomDeidentificationSummary::default().requires_review());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test dicom_workflow_models
```

Expected: FAIL because the DICOM domain types do not exist yet.

- [ ] **Step 3: Write the minimal domain implementation**

Append to `crates/mdid-domain/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DicomPrivateTagPolicy {
    Keep,
    Remove,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnedInAnnotationStatus {
    Clean,
    Suspicious,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DicomTagRef {
    pub group: u16,
    pub element: u16,
    pub keyword: String,
}

impl DicomTagRef {
    pub fn new(group: u16, element: u16, keyword: String) -> Self {
        Self { group, element, keyword }
    }

    pub fn field_path(&self) -> String {
        format!("dicom/{:04x},{:04x}/{}", self.group, self.element, self.keyword)
    }

    pub fn is_private(&self) -> bool {
        self.group % 2 == 1
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DicomPhiCandidate {
    pub tag: DicomTagRef,
    pub phi_type: String,
    pub value: String,
    pub decision: ReviewDecision,
}

impl std::fmt::Debug for DicomPhiCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DicomPhiCandidate")
            .field("tag", &self.tag)
            .field("phi_type", &self.phi_type)
            .field("value", &"<redacted>")
            .field("decision", &self.decision)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DicomDeidentificationSummary {
    pub total_tags: usize,
    pub encoded_tags: usize,
    pub review_required_tags: usize,
    pub removed_private_tags: usize,
    pub remapped_uids: usize,
    pub burned_in_suspicions: usize,
}

impl DicomDeidentificationSummary {
    pub fn requires_review(&self) -> bool {
        self.review_required_tags > 0 || self.burned_in_suspicions > 0
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test dicom_workflow_models
cargo test -p mdid-domain
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/dicom_workflow_models.rs
git commit -m "feat: add dicom workflow domain models"
```

### Task 2: Add DICOM adapter extraction for common PHI tags and private-tag policy classification

**Files:**
- Modify: `crates/mdid-adapters/Cargo.toml`
- Modify: `crates/mdid-adapters/src/lib.rs`
- Create: `crates/mdid-adapters/src/dicom.rs`
- Create: `crates/mdid-adapters/tests/dicom_adapter.rs`

- [ ] **Step 1: Write the failing adapter tests**

Create `crates/mdid-adapters/tests/dicom_adapter.rs` with tests named:
- `extract_identifies_common_phi_tags_and_redacts_debug_output`
- `extract_marks_private_tags_for_review_or_removal_per_policy`
- `extract_flags_burned_in_annotation_as_suspicious`

Use an in-memory DICOM fixture built with:

```rust
let obj = InMemDicomObject::from_element_iter([
    DataElement::new(tags::PATIENT_NAME, VR::PN, "Doe^Jane"),
    DataElement::new(tags::PATIENT_ID, VR::LO, "MRN-42"),
    DataElement::new(tags::BURNED_IN_ANNOTATION, VR::CS, "YES"),
    DataElement::new(Tag(0x0011, 0x0010), VR::LO, "AcmePrivateCreator"),
]);
let file_obj = obj.with_meta(FileMetaTableBuilder::new().transfer_syntax(uids::EXPLICIT_VR_LITTLE_ENDIAN)).unwrap();
let mut bytes = Vec::new();
file_obj.write_all(&mut bytes).unwrap();
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test dicom_adapter
```

Expected: FAIL because the DICOM adapter module and dependency wiring do not exist yet.

- [ ] **Step 3: Write the minimal extraction implementation**

Update `crates/mdid-adapters/Cargo.toml` dependencies:

```toml
dicom-core = "0.9"
dicom-object = "0.9"
```

Create `crates/mdid-adapters/src/dicom.rs` with:

```rust
#[derive(Debug, Clone)]
pub struct DicomAdapter {
    private_tag_policy: DicomPrivateTagPolicy,
}

#[derive(Clone)]
pub struct ExtractedDicomData {
    pub source_name: String,
    pub candidates: Vec<DicomPhiCandidate>,
    pub private_tags: Vec<DicomTagRef>,
    pub burned_in_annotation: BurnedInAnnotationStatus,
}

impl std::fmt::Debug for ExtractedDicomData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedDicomData")
            .field("source_name", &self.source_name)
            .field("candidate_count", &self.candidates.len())
            .field("private_tags", &self.private_tags)
            .field("burned_in_annotation", &self.burned_in_annotation)
            .finish()
    }
}

impl DicomAdapter {
    pub fn new(private_tag_policy: DicomPrivateTagPolicy) -> Self {
        Self { private_tag_policy }
    }

    pub fn extract(&self, bytes: &[u8], source_name: &str) -> Result<ExtractedDicomData, DicomAdapterError> {
        let object = FileDicomObject::from_reader(std::io::Cursor::new(bytes))?;
        // collect PatientName / PatientID / AccessionNumber / StudyDescription when present
        // collect private tags by odd group number
        // treat BurnedInAnnotation == "YES" as Suspicious
    }
}
```

Export it from `crates/mdid-adapters/src/lib.rs`:

```rust
mod dicom;
mod tabular;

pub use dicom::{DicomAdapter, DicomAdapterError, ExtractedDicomData};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test dicom_adapter
cargo test -p mdid-adapters
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-adapters/Cargo.toml crates/mdid-adapters/src/lib.rs crates/mdid-adapters/src/dicom.rs crates/mdid-adapters/tests/dicom_adapter.rs
git commit -m "feat: add dicom adapter extraction"
```

### Task 3: Add DICOM write-back with UID remap and filename/path sanitization

**Files:**
- Modify: `crates/mdid-adapters/src/dicom.rs`
- Modify: `crates/mdid-adapters/tests/dicom_adapter.rs`

- [ ] **Step 1: Write the failing rewrite tests**

Add tests named:
- `rewrite_replaces_encoded_phi_tags_and_remaps_uid_family`
- `rewrite_removes_private_tags_when_policy_is_remove`
- `sanitize_filename_replaces_phi_like_names_with_safe_slug`

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test dicom_adapter rewrite_ -- --nocapture
```

Expected: FAIL because rewrite support does not exist yet.

- [ ] **Step 3: Write the minimal rewrite implementation**

Add to `crates/mdid-adapters/src/dicom.rs`:

```rust
pub struct DicomRewritePlan {
    pub replacements: Vec<(DicomTagRef, String)>,
    pub uid_replacements: Vec<(DicomTagRef, String)>,
}

impl DicomAdapter {
    pub fn rewrite(
        &self,
        bytes: &[u8],
        plan: &DicomRewritePlan,
        sanitized_file_name: &str,
    ) -> Result<Vec<u8>, DicomAdapterError> {
        let mut object = FileDicomObject::from_reader(std::io::Cursor::new(bytes))?;
        // replace planned tags, remove or retain private tags per policy,
        // rewrite StudyInstanceUID / SeriesInstanceUID / SOPInstanceUID from plan,
        // and store sanitized_file_name into a safe output-facing field path helper.
        let mut out = Vec::new();
        object.write_all(&mut out)?;
        Ok(out)
    }
}

pub fn sanitize_output_name(source_name: &str) -> String {
    source_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' { ch } else { '_' })
        .collect()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test dicom_adapter
cargo test -p mdid-adapters
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-adapters/src/dicom.rs crates/mdid-adapters/tests/dicom_adapter.rs
git commit -m "feat: add dicom rewrite and uid remap"
```

### Task 4: Add application orchestration for reversible DICOM de-identification

**Files:**
- Modify: `crates/mdid-application/Cargo.toml`
- Modify: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/dicom_deidentification.rs`

- [ ] **Step 1: Write the failing application tests**

Create tests named:
- `dicom_deidentification_reuses_vault_tokens_for_repeated_phi_values`
- `dicom_deidentification_routes_private_tags_and_burned_in_suspicion_to_review`
- `dicom_deidentification_debug_redacts_output_bytes`

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test dicom_deidentification
```

Expected: FAIL because the DICOM application service does not exist yet.

- [ ] **Step 3: Write the minimal application implementation**

Add to `crates/mdid-application/src/lib.rs`:

```rust
#[derive(Clone)]
pub struct DicomDeidentificationOutput {
    pub bytes: Vec<u8>,
    pub summary: DicomDeidentificationSummary,
    pub review_queue: Vec<DicomPhiCandidate>,
    pub sanitized_file_name: String,
}

impl fmt::Debug for DicomDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DicomDeidentificationOutput")
            .field("bytes", &"[REDACTED]")
            .field("summary", &self.summary)
            .field("review_queue_len", &self.review_queue.len())
            .field("sanitized_file_name", &self.sanitized_file_name)
            .finish()
    }
}

#[derive(Clone, Default)]
pub struct DicomDeidentificationService;
```

Implement `deidentify_bytes(...)` so it:
- extracts candidates through `DicomAdapter`
- uses `LocalVaultStore::ensure_mapping(...)` for approved tags and remapped UIDs
- sends private-tag review items and burned-in suspicion to `review_queue`
- returns honest `DicomDeidentificationSummary`

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test dicom_deidentification
cargo test -p mdid-application
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-application/Cargo.toml crates/mdid-application/src/lib.rs crates/mdid-application/tests/dicom_deidentification.rs
git commit -m "feat: add dicom deidentification service"
```

### Task 5: Truth-sync docs, verify the workspace, and update milestone status

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write the failing doc/verification expectation as a checklist comment in the commit message draft**

Use this exact checklist in your scratch notes before editing:

```text
- README mentions DICOM tag-level support truthfully
- no claim of pixel-level redaction
- workspace tests cover mdid-domain, mdid-adapters, mdid-application, and full workspace
```

- [ ] **Step 2: Update `README.md` with honest Slice 4 status**

Add a status line similar to:

```md
- Slice 4 DICOM support: tag-level PHI handling, UID remap, private-tag policy routing, and burned-in suspicion review flow.
```

- [ ] **Step 3: Run verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain
cargo test -p mdid-adapters
cargo test -p mdid-application
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 4: Merge back to `develop`**

```bash
git checkout develop
git merge --no-ff feature/issue-4-dicom-tag-level-support -m "merge: land slice 4 dicom tag-level support"
source "$HOME/.cargo/env"
cargo test --workspace
```

- [ ] **Step 5: Push and update issue tracking**

```bash
git push origin develop
```

Then add a GitHub comment to issue `#4` summarizing landed adapter, UID remap, review routing, and verification results. If all scope in this plan is complete, close issue `#4`; otherwise keep it open with the remaining gap explicitly listed.
