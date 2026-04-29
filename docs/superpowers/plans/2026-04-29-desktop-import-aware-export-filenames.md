# Desktop Import-Aware Export Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the desktop app suggest PHI-safe de-identified export filenames derived from the imported CSV/XLSX/DICOM source filename instead of only generic names.

**Architecture:** Keep the behavior in the focused desktop library helper layer so the egui shell can consume it without duplicating filename logic. The helper must strip path components, sanitize unsafe characters, remove the original extension, and append a bounded de-identified suffix appropriate to the current desktop workflow mode.

**Tech Stack:** Rust workspace, `mdid-desktop` crate, built-in Rust unit tests, Cargo.

---

## File structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a source-aware export filename helper near `DesktopWorkflowResponseState::suggested_export_file_name`.
  - Preserve the existing generic `suggested_export_file_name` API for current UI callers.
  - Add unit tests in the existing `#[cfg(test)]` module.
- Modify: `README.md`
  - Truth-sync the completion snapshot and verification evidence after the landed desktop filename helper.

### Task 1: Desktop source-aware export filename helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
    #[test]
    fn desktop_export_filename_uses_import_source_stem_for_csv_xlsx_and_dicom() {
        let mut state = DesktopWorkflowResponseState::default();
        state.output = "rewritten payload".to_string();

        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::CsvText,
                Some("/clinic/intake/patient list.csv")
            ),
            Some("patient-list-deidentified.csv".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::XlsxBase64,
                Some("C:\\clinic\\April Census.xlsx")
            ),
            Some("April-Census-deidentified.xlsx.base64.txt".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::DicomBase64,
                Some("brain scan.dcm")
            ),
            Some("brain-scan-deidentified.dcm.base64.txt".to_string())
        );
    }

    #[test]
    fn desktop_export_filename_falls_back_when_source_is_empty_or_unsafe() {
        let mut state = DesktopWorkflowResponseState::default();
        state.output = "rewritten payload".to_string();

        assert_eq!(
            state.suggested_export_file_name_for_source(DesktopWorkflowMode::CsvText, None),
            Some("desktop-deidentified.csv".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(DesktopWorkflowMode::CsvText, Some("///.csv")),
            Some("desktop-deidentified.csv".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::PdfBase64Review,
                Some("report.pdf")
            ),
            None
        );
    }
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-desktop desktop_export_filename -- --nocapture
```

Expected: FAIL with `no method named suggested_export_file_name_for_source`.

- [ ] **Step 3: Implement minimal helper**

In `impl DesktopWorkflowResponseState`, keep the existing `suggested_export_file_name` method and add this method plus private helpers near it:

```rust
    pub fn suggested_export_file_name_for_source(
        &self,
        mode: DesktopWorkflowMode,
        source_name: Option<&str>,
    ) -> Option<String> {
        self.exportable_output()?;
        let fallback = self.suggested_export_file_name(mode)?.to_string();
        let stem = source_name.and_then(safe_source_file_stem);
        let stem = stem.as_deref().unwrap_or("desktop");

        Some(match mode {
            DesktopWorkflowMode::CsvText => format!("{stem}-deidentified.csv"),
            DesktopWorkflowMode::XlsxBase64 => {
                format!("{stem}-deidentified.xlsx.base64.txt")
            }
            DesktopWorkflowMode::PdfBase64Review => return None,
            DesktopWorkflowMode::DicomBase64 => format!("{stem}-deidentified.dcm.base64.txt"),
            DesktopWorkflowMode::MediaMetadataJson => return None,
        })
        .filter(|candidate| candidate != "desktop-deidentified.csv" || fallback == *candidate)
        .or(Some(fallback))
    }
```

Add these private helpers outside the impl:

```rust
fn safe_source_file_stem(source_name: &str) -> Option<String> {
    let filename = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name)
        .trim();
    let stem = filename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(filename)
        .trim();
    let mut safe = String::new();
    let mut last_was_dash = false;

    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            safe.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !safe.is_empty() {
            safe.push('-');
            last_was_dash = true;
        }
    }

    while safe.ends_with('-') {
        safe.pop();
    }

    if safe.is_empty() {
        None
    } else {
        Some(safe)
    }
}
```

If the filter expression is confusing or does not compile, replace the method body with the simpler equivalent:

```rust
        let stem = source_name
            .and_then(safe_source_file_stem)
            .unwrap_or_else(|| "desktop".to_string());

        match mode {
            DesktopWorkflowMode::CsvText => Some(format!("{stem}-deidentified.csv")),
            DesktopWorkflowMode::XlsxBase64 => Some(format!("{stem}-deidentified.xlsx.base64.txt")),
            DesktopWorkflowMode::PdfBase64Review => None,
            DesktopWorkflowMode::DicomBase64 => Some(format!("{stem}-deidentified.dcm.base64.txt")),
            DesktopWorkflowMode::MediaMetadataJson => None,
        }
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-desktop desktop_export_filename -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run broader desktop tests**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-desktop -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): derive export filenames from imports"
```

### Task 2: README truth-sync for desktop filename helper

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update README completion snapshot**

Change the completion snapshot wording to mention the desktop import-aware export filename helper and update truthful percentages only if the landed feature and verification support it:

```markdown
Completion snapshot, based only on landed repository features and verification state (truth-synced 2026-04-29 after the bounded desktop import-aware export filename helper landed and was verified):
```

Update the Desktop app row to mention:

```markdown
... applies bounded CSV/XLSX/PDF/DICOM file import/export helpers with import-aware CSV/XLSX/DICOM export filename suggestions ...
```

Update verification evidence to:

```markdown
Verification evidence for this truth-sync: `RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-desktop -- --nocapture` passed for the landed bounded desktop import-aware export filename helper.
```

- [x] **Step 2: Run README check**

Run:

```bash
grep -n "Completion snapshot\|Desktop app\|Verification evidence" README.md
```

Expected: README mentions desktop import-aware export filename helper and the desktop verification command.

- [x] **Step 3: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-import-aware-export-filenames.md
git commit -m "docs: truth-sync desktop import-aware export filename completion"
```

## Self-review

Task 2 completion note (2026-04-29): README truth-sync now references the landed bounded desktop import-aware export filename helper, keeps desktop completion at 58%, and records the passing `mdid-desktop` verification command.

- Spec coverage: Task 1 implements the helper and tests CSV/XLSX/DICOM/PDF fallback behavior. Task 2 updates README completion and verification evidence.
- Placeholder scan: no TBD/TODO/implement-later placeholders are present.
- Type consistency: `DesktopWorkflowResponseState::suggested_export_file_name_for_source`, `DesktopWorkflowMode`, and existing fallback method names are used consistently.
