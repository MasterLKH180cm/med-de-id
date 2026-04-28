# CSV BOM Header Normalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure UTF-8 BOM-prefixed CSV files match explicit field policies by normalizing the first header before PHI candidate detection.

**Architecture:** Keep the behavior in the tabular adapter boundary where raw CSV headers first enter the system. Normalize only a leading UTF-8 BOM on the first header and leave all other header text unchanged so downstream domain/application/runtime behavior continues to consume the same `ExtractedTabularData` shape.

**Tech Stack:** Rust workspace, `mdid-adapters`, `mdid-domain`, Cargo tests.

---

## File Structure

- Modify `crates/mdid-adapters/tests/csv_tabular_adapter.rs`: add a focused regression test proving a BOM-prefixed first header is exposed without the BOM and still matches `FieldPolicy::encode`.
- Modify `crates/mdid-adapters/src/tabular.rs`: normalize a leading `\u{feff}` on the first CSV/XLSX header through the shared extracted-data path.
- Modify `README.md`: update completion snapshot truthfully; this small import hardening does not change headline completion percentages.

### Task 1: Normalize leading BOM in tabular headers

**Files:**
- Modify: `crates/mdid-adapters/tests/csv_tabular_adapter.rs`
- Modify: `crates/mdid-adapters/src/tabular.rs`

- [ ] **Step 1: Write the failing test**

Add this test to `crates/mdid-adapters/tests/csv_tabular_adapter.rs`:

```rust
#[test]
fn utf8_bom_prefixed_first_header_matches_field_policy() {
    let csv_input = "\u{feff}patient_id,patient_name\nMRN-001,Alice Smith\n";
    let adapter = CsvTabularAdapter::new(vec![FieldPolicy::encode("patient_id", "patient_id")]);

    let extracted = adapter.extract(csv_input.as_bytes()).unwrap();

    assert_eq!(extracted.columns[0].name, "patient_id");
    assert_eq!(extracted.candidates.len(), 1);
    assert_eq!(extracted.candidates[0].cell.header, "patient_id");
    assert_eq!(extracted.candidates[0].value, "MRN-001");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-adapters utf8_bom_prefixed_first_header_matches_field_policy -- --nocapture`

Expected: FAIL because the first column name is `\u{feff}patient_id` and no policy candidate is produced.

- [ ] **Step 3: Write minimal implementation**

In `crates/mdid-adapters/src/tabular.rs`, add this helper near the other private tabular helpers:

```rust
fn normalize_headers(headers: Vec<String>) -> Vec<String> {
    headers
        .into_iter()
        .enumerate()
        .map(|(index, header)| {
            if index == 0 {
                header.trim_start_matches('\u{feff}').to_owned()
            } else {
                header
            }
        })
        .collect()
}
```

Then call it at the start of `build_extracted_data`:

```rust
fn build_extracted_data(
    format: TabularFormat,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    policies: &[FieldPolicy],
) -> ExtractedTabularData {
    let headers = normalize_headers(headers);
    let columns = headers
        .iter()
        .enumerate()
```

- [ ] **Step 4: Run targeted and adapter tests**

Run: `cargo test -p mdid-adapters utf8_bom_prefixed_first_header_matches_field_policy -- --nocapture`
Expected: PASS.

Run: `cargo test -p mdid-adapters`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-adapters/tests/csv_tabular_adapter.rs crates/mdid-adapters/src/tabular.rs
git commit -m "fix(adapters): normalize tabular bom headers"
```

### Task 2: Truth-sync README completion snapshot

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README with a controller-visible completion note**

In `README.md`, keep CLI/browser/desktop/overall percentages unchanged and add a precise implemented-so-far bullet:

```markdown
- CSV/tabular import hardening strips a leading UTF-8 BOM from the first header before policy matching, so BOM-prefixed CSV exports still match explicit field policies; this is a narrow adapter normalization and does not broaden upload/import workflows
```

- [ ] **Step 2: Verify README mentions completion areas**

Run: `grep -n "| CLI |\|| Browser/web |\|| Desktop app |\|| Overall |\|UTF-8 BOM" README.md`
Expected: lines for CLI, Browser/web, Desktop app, Overall, and UTF-8 BOM note.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-28-csv-bom-header-normalization.md
git commit -m "docs: update csv bom completion snapshot"
```
