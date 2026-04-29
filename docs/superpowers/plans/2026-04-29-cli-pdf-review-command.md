# CLI PDF Review Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli deidentify-pdf` command that reads a local PDF, routes it through the existing PDF review-only application service, and writes a PHI-safe JSON review report.

**Architecture:** Keep the CLI thin: parse local paths/source name, read bytes, delegate to `mdid_application::PdfDeidentificationService::deidentify_bytes`, and write a JSON report containing only aggregate summary/page statuses/review count/rewrite availability. Do not add OCR, visual redaction, PDF rewrite/export, vault behavior, browser/desktop workflow behavior, or any controller/agent semantics.

**Tech Stack:** Rust, existing `mdid-cli`, `mdid-application`, `mdid-adapters` PDF foundation, serde JSON, Cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add parse/run support for `deidentify-pdf`, PHI-safe report writing, usage text, and unit parse test.
- Create: `crates/mdid-cli/tests/cli_pdf.rs` — command-level smoke tests for valid text-layer PDF report and invalid PDF error.
- Modify: `README.md` — truth-sync CLI/Overall completion and limitations after tests pass.
- Modify: `docs/superpowers/plans/2026-04-29-cli-pdf-review-command.md` — mark completed checklist items after implementation.

## Task 1: Add bounded CLI PDF review command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_pdf.rs`

- [x] **Step 1: Write failing parse test**

Add this test inside `#[cfg(test)] mod tests` in `crates/mdid-cli/src/main.rs`:

```rust
#[test]
fn parses_deidentify_pdf_command_without_requiring_debug() {
    let args = vec![
        "deidentify-pdf".to_string(),
        "--pdf-path".to_string(),
        "input.pdf".to_string(),
        "--source-name".to_string(),
        "scan.pdf".to_string(),
        "--report-path".to_string(),
        "report.json".to_string(),
    ];

    assert!(
        parse_command(&args)
            == Ok(CliCommand::DeidentifyPdf(DeidentifyPdfArgs {
                pdf_path: PathBuf::from("input.pdf"),
                source_name: "scan.pdf".to_string(),
                report_path: PathBuf::from("report.json"),
            }))
    );
}
```

- [x] **Step 2: Run parse test and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli parses_deidentify_pdf_command_without_requiring_debug -- --nocapture
```

Expected: FAIL because `DeidentifyPdf`, `DeidentifyPdfArgs`, and parser support do not exist.

- [x] **Step 3: Implement minimal parser and usage support**

In `crates/mdid-cli/src/main.rs`:

```rust
use mdid_application::{DicomDeidentificationService, PdfDeidentificationService, TabularDeidentificationService};
```

Add enum variant and args struct:

```rust
DeidentifyPdf(DeidentifyPdfArgs),

#[derive(Clone, PartialEq, Eq)]
struct DeidentifyPdfArgs {
    pdf_path: PathBuf,
    source_name: String,
    report_path: PathBuf,
}
```

Add parse branch:

```rust
[command, rest @ ..] if command == "deidentify-pdf" => {
    parse_deidentify_pdf_args(rest).map(CliCommand::DeidentifyPdf)
}
```

Add parser:

```rust
fn parse_deidentify_pdf_args(args: &[String]) -> Result<DeidentifyPdfArgs, String> {
    let mut pdf_path = None;
    let mut source_name = None;
    let mut report_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--pdf-path" => pdf_path = Some(PathBuf::from(value)),
            "--source-name" => source_name = Some(value.clone()),
            "--report-path" => report_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let source_name = source_name.ok_or_else(|| "missing --source-name".to_string())?;
    if source_name.trim().is_empty() {
        return Err("missing --source-name".to_string());
    }

    Ok(DeidentifyPdfArgs {
        pdf_path: pdf_path.ok_or_else(|| "missing --pdf-path".to_string())?,
        source_name,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
    })
}
```

Add run dispatch placeholder after parse compiles:

```rust
CliCommand::DeidentifyPdf(args) => run_deidentify_pdf(args),
```

- [x] **Step 4: Run parse test and verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli parses_deidentify_pdf_command_without_requiring_debug -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Write failing command smoke tests**

Create `crates/mdid-cli/tests/cli_pdf.rs`:

```rust
use std::{fs, process::Command};

use tempfile::tempdir;

fn minimal_text_pdf(text: &str) -> Vec<u8> {
    let stream = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
    let obj1 = b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n";
    let obj2 = b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n";
    let obj3 = b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n";
    let obj4 = format!(
        "4 0 obj << /Length {} >> stream\n{}\nendstream endobj\n",
        stream.len(), stream
    );
    let obj5 = b"5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj\n";

    let mut pdf = b"%PDF-1.4\n".to_vec();
    pdf.extend_from_slice(obj1);
    pdf.extend_from_slice(obj2);
    pdf.extend_from_slice(obj3);
    pdf.extend_from_slice(obj4.as_bytes());
    pdf.extend_from_slice(obj5);
    pdf.extend_from_slice(b"trailer << /Root 1 0 R >>\n%%EOF\n");
    pdf
}

#[test]
fn cli_deidentify_pdf_writes_phi_safe_review_report() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("patient-jane.pdf");
    let report_path = dir.path().join("report.json");
    fs::write(&pdf_path, minimal_text_pdf("Jane Patient MRN123")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("patient-jane.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Jane Patient"));
    assert!(!stdout.contains("MRN123"));
    assert!(stdout.contains("review_queue_len"));

    let report = fs::read_to_string(&report_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(json["rewrite_available"], false);
    assert_eq!(json["rewritten_pdf_bytes"], serde_json::Value::Null);
    assert!(json["summary"].is_object());
    assert!(json["page_statuses"].is_array());
    assert!(json["review_queue_len"].as_u64().unwrap() >= 1);
    assert!(!report.contains("Jane Patient"));
    assert!(!report.contains("MRN123"));
}

#[test]
fn cli_deidentify_pdf_rejects_invalid_pdf_without_scope_drift_terms() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("bad.pdf");
    let report_path = dir.path().join("report.json");
    fs::write(&pdf_path, b"not a pdf").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("bad.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to review PDF"));
    for forbidden in ["moat", "controller", "agent", "orchestration"] {
        assert!(!stderr.to_lowercase().contains(forbidden));
    }
    assert!(!report_path.exists());
}
```

- [x] **Step 6: Run command tests and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test cli_pdf -- --nocapture
```

Expected: FAIL because `run_deidentify_pdf` does not yet write the report.

- [x] **Step 7: Implement minimal PDF review command**

In `crates/mdid-cli/src/main.rs`, add:

```rust
fn run_deidentify_pdf(args: DeidentifyPdfArgs) -> Result<(), String> {
    let bytes = fs::read(&args.pdf_path).map_err(|err| format!("failed to read PDF: {err}"))?;
    let output = PdfDeidentificationService
        .deidentify_bytes(&bytes, args.source_name.trim())
        .map_err(|err| format!("failed to review PDF: {err}"))?;

    let report = json!({
        "summary": output.summary,
        "page_statuses": output.page_statuses,
        "review_queue_len": output.review_queue.len(),
        "rewrite_available": output.rewritten_pdf_bytes.is_some(),
        "rewritten_pdf_bytes": serde_json::Value::Null,
    });
    fs::write(
        &args.report_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to render PDF report: {err}"))?,
    )
    .map_err(|err| format!("failed to write PDF report: {err}"))?;

    let stdout = json!({
        "report_path": args.report_path,
        "summary": report["summary"].clone(),
        "review_queue_len": report["review_queue_len"].clone(),
        "rewrite_available": false,
    });
    println!(
        "{}",
        serde_json::to_string(&stdout)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}
```

Update `usage()` to include:

```text
       mdid-cli deidentify-pdf --pdf-path <input.pdf> --source-name <name.pdf> --report-path <report.json>
  deidentify-pdf      Review a bounded local PDF and write a PHI-safe JSON report; no OCR or PDF rewrite/export.
```

- [x] **Step 8: Run targeted and broader CLI verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli parses_deidentify_pdf_command_without_requiring_debug -- --nocapture
cargo test -p mdid-cli --test cli_pdf -- --nocapture
cargo test -p mdid-cli --all-targets
cargo clippy -p mdid-cli --all-targets -- -D warnings
git diff --check
```

Expected: PASS.

- [x] **Step 9: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_pdf.rs docs/superpowers/plans/2026-04-29-cli-pdf-review-command.md
git commit -m "feat(cli): add bounded pdf review command"
```

## Task 2: README completion truth-sync and final integration

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-29-cli-pdf-review-command.md`

- [x] **Step 1: Update README completion based on landed tests**

Update `README.md` completion table:
- CLI: `72%`, mentioning bounded `deidentify-pdf` review report in addition to CSV/XLSX/DICOM commands, and still listing vault/decode, audit, conservative-media, and import/export CLI commands as missing.
- Overall: `60%`, mentioning CLI PDF review report as a narrow increment; do not claim OCR, visual redaction, PDF rewrite/export, or >90% completion.
- Missing items must still include richer browser upload/download UX depth, deeper desktop vault/decode/audit workflows, OCR, visual redaction, PDF rewrite/export, portable transfer UX, and broader import/export.

- [x] **Step 2: Verify docs and tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --all-targets
cargo test -p mdid-application --test pdf_deidentification
cargo clippy -p mdid-cli --all-targets -- -D warnings
git diff --check
grep -nE 'CLI|Browser/web|Desktop app|Overall|Missing|OCR|visual redaction|PDF rewrite|controller|orchestration|agent|moat' README.md
```

Expected: PASS; grep output shows only honest limitations/scope-drift warnings.

- [x] **Step 3: Mark plan checkboxes complete and commit docs**

```bash
git add README.md docs/superpowers/plans/2026-04-29-cli-pdf-review-command.md
git commit -m "docs: truth-sync cli pdf review completion"
```

## Self-Review

- Spec coverage: This plan covers a bounded CLI PDF review command, PHI-safe report/stdout, invalid PDF behavior, README completion maintenance, and explicit exclusion of OCR/PDF rewrite/scope-drift semantics.
- Placeholder scan: No TBD/TODO placeholders are present; all tests and commands are explicit.
- Type consistency: `DeidentifyPdfArgs`, `parse_deidentify_pdf_args`, `run_deidentify_pdf`, `review_queue_len`, `rewrite_available`, and `rewritten_pdf_bytes` are named consistently across tasks.
