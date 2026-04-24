# med-de-id Slice 3 Tabular Deep Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first CSV/XLSX tabular adapter path with schema inference, field-level PHI review decisions, vault-backed reversible encoding, and honest batch summaries for partial failures.

**Architecture:** This slice adds a focused `mdid-adapters` crate with a tabular module that normalizes CSV/XLSX inputs into one shared in-memory representation. The application layer composes that adapter with `mdid-vault` so repeated PHI values reuse the same token while each cell keeps its own scoped mapping record, approved cells are encoded, review-required cells stay explicit, and the run returns batch summaries instead of pretending every row succeeded.

**Tech Stack:** Rust workspace, Cargo, Serde, Chrono, UUID, thiserror, csv, calamine, rust_xlsxwriter, tempfile.

---

## Scope note

This plan covers **Slice 3 — CSV/Excel deep support** only. It does not implement DICOM, PDF/OCR, or image/video/FCS adapters. To keep scope truthful and shippable, the slice lands tabular support in five narrow tasks:

1. tabular workflow/domain vocabulary
2. vault token-reuse support for repeated PHI values while preserving per-cell scope provenance
3. CSV adapter + schema inference
4. application orchestration + batch summary flow
5. XLSX parity on the same tabular engine

## File structure

**Create:**
- `crates/mdid-domain/tests/tabular_workflow_models.rs`
- `crates/mdid-adapters/Cargo.toml`
- `crates/mdid-adapters/src/lib.rs`
- `crates/mdid-adapters/src/tabular.rs`
- `crates/mdid-adapters/tests/csv_tabular_adapter.rs`
- `crates/mdid-adapters/tests/xlsx_tabular_adapter.rs`
- `crates/mdid-application/tests/tabular_deidentification.rs`

**Modify:**
- `Cargo.toml`
- `README.md`
- `crates/mdid-domain/src/lib.rs`
- `crates/mdid-vault/src/lib.rs`
- `crates/mdid-vault/tests/local_vault_store.rs`
- `crates/mdid-application/Cargo.toml`
- `crates/mdid-application/src/lib.rs`

---

### Task 1: Add tabular workflow vocabulary to `mdid-domain`

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/tabular_workflow_models.rs`

- [ ] **Step 1: Write the failing domain tests**

Create `crates/mdid-domain/tests/tabular_workflow_models.rs`:

```rust
use mdid_domain::{
    BatchSummary, PhiCandidate, ReviewDecision, TabularCellRef, TabularColumn, TabularFormat,
};

#[test]
fn tabular_cell_ref_builds_a_stable_field_path() {
    let cell = TabularCellRef::new(3, 1, "patient/name".into());
    assert_eq!(cell.field_path(), "rows/3/columns/1/patient_name");
}

#[test]
fn review_decision_reports_when_manual_review_is_required() {
    assert!(ReviewDecision::NeedsReview.requires_human_review());
    assert!(!ReviewDecision::Approved.requires_human_review());
    assert!(ReviewDecision::Approved.allows_encode());
    assert!(!ReviewDecision::Rejected.allows_encode());
}

#[test]
fn batch_summary_flags_partial_failure_when_any_rows_fail() {
    let summary = BatchSummary {
        total_rows: 12,
        encoded_cells: 9,
        review_required_cells: 2,
        failed_rows: 1,
    };

    assert!(summary.is_partial_failure());
}

#[test]
fn phi_candidate_debug_redacts_source_value() {
    let candidate = PhiCandidate {
        format: TabularFormat::Csv,
        column: TabularColumn::new(1, "patient_name".into(), "string".into()),
        cell: TabularCellRef::new(1, 1, "patient_name".into()),
        phi_type: "patient_name".into(),
        value: "Alice Smith".into(),
        confidence: 98,
        decision: ReviewDecision::NeedsReview,
    };

    let debug = format!("{candidate:?}");
    assert!(debug.contains("PhiCandidate"));
    assert!(!debug.contains("Alice Smith"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test tabular_workflow_models
```

Expected: FAIL because the tabular domain types do not exist yet.

- [ ] **Step 3: Write the minimal domain implementation**

Append to `crates/mdid-domain/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabularFormat {
    Csv,
    Xlsx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularColumn {
    pub index: usize,
    pub name: String,
    pub inferred_kind: String,
}

impl TabularColumn {
    pub fn new(index: usize, name: String, inferred_kind: String) -> Self {
        Self {
            index,
            name,
            inferred_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularCellRef {
    pub row_index: usize,
    pub column_index: usize,
    pub header: String,
}

impl TabularCellRef {
    pub fn new(row_index: usize, column_index: usize, header: String) -> Self {
        Self {
            row_index,
            column_index,
            header,
        }
    }

    pub fn field_path(&self) -> String {
        format!(
            "rows/{}/columns/{}/{}",
            self.row_index,
            self.column_index,
            self.header.replace('/', "_")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approved,
    Rejected,
    NeedsReview,
}

impl ReviewDecision {
    pub fn allows_encode(&self) -> bool {
        matches!(self, ReviewDecision::Approved)
    }

    pub fn requires_human_review(&self) -> bool {
        matches!(self, ReviewDecision::NeedsReview)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PhiCandidate {
    pub format: TabularFormat,
    pub column: TabularColumn,
    pub cell: TabularCellRef,
    pub phi_type: String,
    pub value: String,
    pub confidence: u8,
    pub decision: ReviewDecision,
}

impl std::fmt::Debug for PhiCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhiCandidate")
            .field("format", &self.format)
            .field("column", &self.column)
            .field("cell", &self.cell)
            .field("phi_type", &self.phi_type)
            .field("value", &"<redacted>")
            .field("confidence", &self.confidence)
            .field("decision", &self.decision)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BatchSummary {
    pub total_rows: usize,
    pub encoded_cells: usize,
    pub review_required_cells: usize,
    pub failed_rows: usize,
}

impl BatchSummary {
    pub fn is_partial_failure(&self) -> bool {
        self.failed_rows > 0
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test tabular_workflow_models
cargo test -p mdid-domain
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/tabular_workflow_models.rs
git commit -m "feat: add tabular workflow domain models"
```

### Task 2: Reuse existing vault tokens for repeated PHI values while preserving per-scope records

**Files:**
- Modify: `crates/mdid-vault/src/lib.rs`
- Modify: `crates/mdid-vault/tests/local_vault_store.rs`

- [ ] **Step 1: Write the failing vault tests**

Add to `crates/mdid-vault/tests/local_vault_store.rs`:

```rust
#[test]
fn ensure_mapping_reuses_existing_record_for_same_scope_phi_type_and_value() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();

    let first_scope = sample_scope("rows/1/columns/0/patient_id");
    let first = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: first_scope.clone(),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    let second = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: first_scope,
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.scope, second.scope);
    assert_eq!(first.token, second.token);
    assert_eq!(vault.audit_events().len(), 1);
}

#[test]
fn ensure_mapping_reuses_token_but_creates_a_new_record_for_a_new_scope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();

    let first = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: sample_scope("rows/1/columns/0/patient_id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    let second = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: sample_scope("rows/2/columns/0/patient_id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    assert_ne!(first.id, second.id);
    assert_ne!(first.scope, second.scope);
    assert_eq!(first.token, second.token);
    assert_eq!(vault.audit_events().len(), 2);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-vault --test local_vault_store ensure_mapping_reuses_existing_record_for_same_scope_phi_type_and_value -- --exact
cargo test -p mdid-vault --test local_vault_store ensure_mapping_reuses_token_but_creates_a_new_record_for_a_new_scope -- --exact
```

Expected: the new-scope test FAILS because the first implementation incorrectly returns the first scoped record instead of creating a new record with the reused token.

- [ ] **Step 3: Write the minimal vault implementation**

Update `crates/mdid-vault/src/lib.rs` so `ensure_mapping` first reuses an exact existing `(scope, phi_type, original_value)` record, then reuses only the token for later scopes with the same `(phi_type, original_value)`:

```rust
impl LocalVaultStore {
    pub fn ensure_mapping(
        &mut self,
        record: NewMappingRecord,
        actor: SurfaceKind,
    ) -> Result<MappingRecord, VaultError> {
        if let Some(existing) =
            self.find_mapping(&record.scope, &record.phi_type, &record.original_value)
        {
            return Ok(existing);
        }

        let reusable_token = self
            .find_mapping_by_value(&record.phi_type, &record.original_value)
            .map(|record| record.token);

        self.store_mapping_with_token(record, actor, reusable_token)
    }

    fn store_mapping_with_token(
        &mut self,
        record: NewMappingRecord,
        actor: SurfaceKind,
        reusable_token: Option<String>,
    ) -> Result<MappingRecord, VaultError> {
        let stored = MappingRecord {
            id: Uuid::new_v4(),
            scope: record.scope,
            phi_type: record.phi_type,
            token: reusable_token.unwrap_or_else(|| format!("tok-{}", Uuid::new_v4().simple())),
            original_value: record.original_value,
            created_at: Utc::now(),
        };

        let mut staged_state = self.state.clone();
        staged_state.records.push(stored.clone());
        staged_state.audit_events.push(AuditEvent {
            id: Uuid::new_v4(),
            kind: AuditEventKind::Encode,
            actor,
            detail: format!("encoded mapping {}", stored.scope.scope_key()),
            recorded_at: Utc::now(),
        });
        self.flush_state(&staged_state)?;
        self.state = staged_state;

        Ok(stored)
    }

    fn find_mapping(
        &self,
        scope: &MappingScope,
        phi_type: &str,
        original_value: &str,
    ) -> Option<MappingRecord> {
        self.state
            .records
            .iter()
            .find(|record| {
                &record.scope == scope
                    && record.phi_type == phi_type
                    && record.original_value == original_value
            })
            .cloned()
    }

    fn find_mapping_by_value(&self, phi_type: &str, original_value: &str) -> Option<MappingRecord> {
        self.state
            .records
            .iter()
            .find(|record| record.phi_type == phi_type && record.original_value == original_value)
            .cloned()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-vault --test local_vault_store ensure_mapping_reuses_existing_record_for_same_scope_phi_type_and_value -- --exact
cargo test -p mdid-vault --test local_vault_store ensure_mapping_reuses_token_but_creates_a_new_record_for_a_new_scope -- --exact
cargo test -p mdid-vault
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-vault/src/lib.rs crates/mdid-vault/tests/local_vault_store.rs
git commit -m "fix: preserve mapping scope when reusing vault tokens"
```

### Task 3: Add the `mdid-adapters` tabular CSV adapter and schema inference

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/mdid-adapters/Cargo.toml`
- Create: `crates/mdid-adapters/src/lib.rs`
- Create: `crates/mdid-adapters/src/tabular.rs`
- Create: `crates/mdid-adapters/tests/csv_tabular_adapter.rs`

- [ ] **Step 1: Write the failing adapter tests**

Create `crates/mdid-adapters/tests/csv_tabular_adapter.rs`:

```rust
use mdid_adapters::{CsvTabularAdapter, FieldPolicy, FieldPolicyAction};

#[test]
fn csv_adapter_infers_schema_and_marks_review_columns() {
    let csv_input = "patient_id,patient_name,age\nMRN-001,Alice Smith,42\n";
    let adapter = CsvTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = adapter.extract(csv_input.as_bytes()).unwrap();

    assert_eq!(extracted.columns.len(), 3);
    assert_eq!(extracted.columns[0].name, "patient_id");
    assert_eq!(extracted.columns[2].inferred_kind, "integer");
    assert_eq!(extracted.candidates.len(), 2);
    assert_eq!(extracted.candidates[0].decision, mdid_domain::ReviewDecision::Approved);
    assert_eq!(extracted.candidates[1].decision, mdid_domain::ReviewDecision::NeedsReview);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test csv_tabular_adapter
```

Expected: FAIL because the crate and adapter do not exist yet.

- [ ] **Step 3: Write the minimal adapter implementation**

Update workspace `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/mdid-domain",
  "crates/mdid-vault",
  "crates/mdid-adapters",
  "crates/mdid-application",
  "crates/mdid-runtime",
  "crates/mdid-cli",
  "crates/mdid-browser",
  "crates/mdid-desktop",
]

[workspace.dependencies]
csv = "1.3"
calamine = "0.25"
rust_xlsxwriter = "0.79"
```

Create `crates/mdid-adapters/Cargo.toml`:

```toml
[package]
name = "mdid-adapters"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
csv.workspace = true
mdid-domain = { path = "../mdid-domain" }
serde.workspace = true
thiserror.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Create `crates/mdid-adapters/src/lib.rs`:

```rust
mod tabular;

pub use tabular::{CsvTabularAdapter, ExtractedTabularData, FieldPolicy, FieldPolicyAction};
```

Create the core of `crates/mdid-adapters/src/tabular.rs`:

```rust
use mdid_domain::{PhiCandidate, ReviewDecision, TabularCellRef, TabularColumn, TabularFormat};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldPolicyAction {
    Encode,
    Review,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPolicy {
    pub header: String,
    pub phi_type: String,
    pub action: FieldPolicyAction,
}

impl FieldPolicy {
    pub fn encode(header: &str, phi_type: &str) -> Self {
        Self {
            header: header.into(),
            phi_type: phi_type.into(),
            action: FieldPolicyAction::Encode,
        }
    }

    pub fn review(header: &str, phi_type: &str) -> Self {
        Self {
            header: header.into(),
            phi_type: phi_type.into(),
            action: FieldPolicyAction::Review,
        }
    }
}

pub struct ExtractedTabularData {
    pub format: TabularFormat,
    pub columns: Vec<TabularColumn>,
    pub rows: Vec<Vec<String>>,
    pub candidates: Vec<PhiCandidate>,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test csv_tabular_adapter
cargo test -p mdid-adapters
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/mdid-adapters
git commit -m "feat: add csv tabular adapter and schema inference"
```

### Task 4: Add application-level CSV de-identification orchestration

**Files:**
- Modify: `crates/mdid-application/Cargo.toml`
- Modify: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/tabular_deidentification.rs`

- [ ] **Step 1: Write the failing application tests**

Create `crates/mdid-application/tests/tabular_deidentification.rs`:

```rust
use mdid_adapters::FieldPolicy;
use mdid_application::TabularDeidentificationService;
use mdid_domain::{ReviewDecision, SurfaceKind};
use mdid_vault::LocalVaultStore;
use tempfile::tempdir;

#[test]
fn csv_deidentification_reuses_tokens_and_reports_review_items() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = TabularDeidentificationService::default();
    let policies = vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ];

    let output = service
        .deidentify_csv(
            "patient_id,patient_name\nMRN-001,Alice Smith\nMRN-001,Alice Smith\n",
            &policies,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();

    let lines = output.csv.lines().collect::<Vec<_>>();
    assert_eq!(lines[1], lines[2]);
    assert_eq!(output.summary.total_rows, 2);
    assert_eq!(output.summary.encoded_cells, 2);
    assert_eq!(output.summary.review_required_cells, 2);
    assert_eq!(output.review_queue.len(), 2);
    assert!(output.review_queue.iter().all(|candidate| candidate.decision == ReviewDecision::NeedsReview));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test tabular_deidentification
```

Expected: FAIL because the new service does not exist yet.

- [ ] **Step 3: Write the minimal application implementation**

Update `crates/mdid-application/Cargo.toml`:

```toml
[dependencies]
chrono.workspace = true
mdid-adapters = { path = "../mdid-adapters" }
mdid-domain = { path = "../mdid-domain" }
mdid-vault = { path = "../mdid-vault" }
thiserror.workspace = true
uuid.workspace = true
```

Add to `crates/mdid-application/src/lib.rs`:

```rust
#[derive(Debug, Clone)]
pub struct TabularDeidentificationOutput {
    pub csv: String,
    pub summary: BatchSummary,
    pub review_queue: Vec<PhiCandidate>,
}

#[derive(Clone, Default)]
pub struct TabularDeidentificationService;
```

Implement the core of `deidentify_csv(...)` in `crates/mdid-application/src/lib.rs`:

```rust
impl TabularDeidentificationService {
    pub fn deidentify_csv(
        &self,
        csv: &str,
        policies: &[FieldPolicy],
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<TabularDeidentificationOutput, ApplicationError> {
        let adapter = CsvTabularAdapter::new(policies.to_vec());
        let extracted = adapter.extract(csv.as_bytes())?;
        let mut rewritten_rows = Vec::with_capacity(extracted.rows.len());
        let mut summary = BatchSummary {
            total_rows: extracted.rows.len(),
            ..BatchSummary::default()
        };
        let mut review_queue = Vec::new();

        for candidate in extracted.candidates.iter().cloned() {
            if candidate.decision.requires_human_review() {
                summary.review_required_cells += 1;
                review_queue.push(candidate);
                continue;
            }

            let mapping = vault.ensure_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(Uuid::new_v4(), Uuid::new_v4(), candidate.cell.field_path()),
                    phi_type: candidate.phi_type.clone(),
                    original_value: candidate.value.clone(),
                },
                actor,
            )?;

            summary.encoded_cells += 1;
            rewritten_rows.push((candidate.cell.row_index, candidate.cell.column_index, mapping.token));
        }

        Ok(TabularDeidentificationOutput {
            csv: CsvTabularAdapter::rewrite_csv(csv, &rewritten_rows)?,
            summary,
            review_queue,
        })
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test tabular_deidentification
cargo test -p mdid-application
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-application/Cargo.toml crates/mdid-application/src/lib.rs crates/mdid-application/tests/tabular_deidentification.rs
git commit -m "feat: orchestrate csv deidentification through the vault"
```

### Task 5: Add XLSX parity on the same tabular engine and refresh docs

**Files:**
- Modify: `crates/mdid-adapters/src/tabular.rs`
- Create: `crates/mdid-adapters/tests/xlsx_tabular_adapter.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing XLSX tests**

Create `crates/mdid-adapters/tests/xlsx_tabular_adapter.rs`:

```rust
use mdid_adapters::{FieldPolicy, XlsxTabularAdapter};

#[test]
fn xlsx_adapter_reads_headers_and_preserves_row_count() {
    let workbook = XlsxTabularAdapter::fixture_bytes(vec![
        vec!["patient_id", "patient_name"],
        vec!["MRN-001", "Alice Smith"],
        vec!["MRN-002", "Bob Jones"],
    ]);
    let adapter = XlsxTabularAdapter::new(vec![FieldPolicy::encode("patient_id", "patient_id")]);

    let extracted = adapter.extract(&workbook).unwrap();

    assert_eq!(extracted.columns.len(), 2);
    assert_eq!(extracted.rows.len(), 2);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test xlsx_tabular_adapter
```

Expected: FAIL because the XLSX adapter does not exist yet.

- [ ] **Step 3: Write the minimal XLSX implementation and docs**

Add `XlsxTabularAdapter` beside the CSV adapter in `crates/mdid-adapters/src/tabular.rs`:

```rust
pub struct XlsxTabularAdapter {
    policies: Vec<FieldPolicy>,
}

impl XlsxTabularAdapter {
    pub fn new(policies: Vec<FieldPolicy>) -> Self {
        Self { policies }
    }

    pub fn extract(&self, bytes: &[u8]) -> Result<ExtractedTabularData, TabularAdapterError> {
        let workbook = calamine::open_workbook_from_rs(std::io::Cursor::new(bytes.to_vec()))?;
        extract_first_sheet(workbook, &self.policies)
    }

    pub fn fixture_bytes(rows: Vec<Vec<&str>>) -> Vec<u8> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                worksheet
                    .write_string(row_index as u32, column_index as u16, value)
                    .unwrap();
            }
        }
        workbook.save_to_buffer().unwrap()
    }
}
```

Update the status section in `README.md`:

```md
## Current repository status

This repository currently contains the Slice 1 workspace foundation, the Slice 2 vault MVP, and the first Slice 3 tabular adapter work in progress.

Implemented so far:

- Shared domain models for pipeline, review, vault mapping, decode requests, audit events, and tabular workflow state
- An encrypted `mdid-vault` crate with local file-backed storage, explicit decode-by-record-id, audit recording, portable subset export, and repeated-value token reuse
- Initial CSV/XLSX tabular adapter coverage for schema inference and reversible field-level encoding
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test xlsx_tabular_adapter
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-adapters/src/tabular.rs crates/mdid-adapters/tests/xlsx_tabular_adapter.rs README.md
git commit -m "feat: add xlsx tabular adapter parity"
```

## Self-review

### Spec coverage
- schema inference → Task 3
- field-level PHI policies → Tasks 3 and 4
- consistent tokenization across rows/files with per-scope provenance → Task 2, then consumed in Task 4
- review/override path → Tasks 1 and 4
- decode compatibility via vault-backed mappings → Task 2 and Task 4
- batch summaries and partial-failure reporting → Tasks 1 and 4
- Excel parity → Task 5

### Placeholder scan
- Replaced vague phrases like “add validation” with concrete methods/tests (`requires_human_review`, `ensure_mapping`, `is_partial_failure`)
- Every task includes exact file paths and explicit test commands

### Type consistency
- The plan consistently uses `TabularFormat`, `TabularColumn`, `TabularCellRef`, `PhiCandidate`, `ReviewDecision`, `BatchSummary`, `FieldPolicy`, `CsvTabularAdapter`, and `TabularDeidentificationService`
- Vault token reuse is consistently named `ensure_mapping`
