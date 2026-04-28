# CLI Vault Decode Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli vault-decode` command that unlocks a local vault, decodes explicit record ids, records the vault audit event, and writes an explicit JSON decode report.

**Architecture:** Keep the CLI as the automation surface and reuse the existing `mdid-vault::LocalVaultStore::decode` plus `mdid_domain::DecodeRequest` contract. The command is intentionally local and bounded: no vault browsing UI, no workflow orchestration, no agent/controller semantics, and no generalized transfer behavior.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-domain`, `mdid-vault`, `serde_json`, Cargo tests.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add `VaultDecode` CLI command parsing, runner, PHI-containing report writing, PHI-safe stdout summary, usage text, and focused unit tests in the existing test module.
- Modify `README.md`: update completion ledger and CLI capability text after the feature lands and tests pass.

### Task 1: Bounded CLI vault-decode command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/src/main.rs` existing `#[cfg(test)]` module

- [x] **Step 1: Write the failing parser test**

Add this test to the existing test module in `crates/mdid-cli/src/main.rs`:

```rust
#[test]
fn parses_vault_decode_command() {
    let args = vec![
        "vault-decode".to_string(),
        "--vault-path".to_string(),
        "vault.json".to_string(),
        "--passphrase".to_string(),
        "secret".to_string(),
        "--record-ids-json".to_string(),
        r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
        "--output-target".to_string(),
        "case review packet".to_string(),
        "--justification".to_string(),
        "approved disclosure".to_string(),
        "--report-path".to_string(),
        "decode-report.json".to_string(),
    ];

    let command = parse_command(&args).expect("vault decode command parses");

    match command {
        CliCommand::VaultDecode(parsed) => {
            assert_eq!(parsed.vault_path, PathBuf::from("vault.json"));
            assert_eq!(parsed.passphrase, "secret");
            assert_eq!(
                parsed.record_ids_json,
                r#"["550e8400-e29b-41d4-a716-446655440000"]"#
            );
            assert_eq!(parsed.output_target, "case review packet");
            assert_eq!(parsed.justification, "approved disclosure");
            assert_eq!(parsed.report_path, PathBuf::from("decode-report.json"));
        }
        _ => panic!("expected vault decode command"),
    }
}
```

- [x] **Step 2: Run parser test to verify RED**

Run: `cargo test -p mdid-cli parses_vault_decode_command -- --nocapture`

Expected: FAIL because `CliCommand::VaultDecode` and parser support do not exist yet.

- [x] **Step 3: Add minimal parser implementation**

In `crates/mdid-cli/src/main.rs`, add:

```rust
use uuid::Uuid;
```

Extend `CliCommand`:

```rust
    VaultDecode(VaultDecodeArgs),
```

Add args struct:

```rust
#[derive(Clone, PartialEq, Eq)]
struct VaultDecodeArgs {
    vault_path: PathBuf,
    passphrase: String,
    record_ids_json: String,
    output_target: String,
    justification: String,
    report_path: PathBuf,
}
```

Add parse branch before `vault-audit` or after it:

```rust
        [command, rest @ ..] if command == "vault-decode" => {
            parse_vault_decode_args(rest).map(CliCommand::VaultDecode)
        }
```

Add parser function:

```rust
fn parse_vault_decode_args(args: &[String]) -> Result<VaultDecodeArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut record_ids_json = None;
    let mut output_target = None;
    let mut justification = None;
    let mut report_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--record-ids-json" => record_ids_json = Some(value.clone()),
            "--output-target" => output_target = Some(value.clone()),
            "--justification" => justification = Some(value.clone()),
            "--report-path" => report_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let output_target = output_target.ok_or_else(|| "missing --output-target".to_string())?;
    if output_target.trim().is_empty() {
        return Err("missing --output-target".to_string());
    }
    let justification = justification.ok_or_else(|| "missing --justification".to_string())?;
    if justification.trim().is_empty() {
        return Err("missing --justification".to_string());
    }

    Ok(VaultDecodeArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        record_ids_json: record_ids_json.ok_or_else(|| "missing --record-ids-json".to_string())?,
        output_target,
        justification,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
    })
}
```

- [x] **Step 4: Run parser test to verify GREEN**

Run: `cargo test -p mdid-cli parses_vault_decode_command -- --nocapture`

Expected: PASS.

- [x] **Step 5: Write failing behavior tests**

Add tests:

```rust
#[test]
fn parses_record_ids_json_for_vault_decode() {
    let record_ids = parse_record_ids_json(
        r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440001"]"#,
    )
    .expect("valid record ids json");

    assert_eq!(record_ids.len(), 2);
    assert_eq!(
        record_ids[0].to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}

#[test]
fn rejects_empty_record_ids_json_for_vault_decode() {
    let error = parse_record_ids_json("[]").expect_err("empty decode scope rejected");

    assert_eq!(error, "decode scope must include at least one record id");
}

#[test]
fn vault_decode_report_keeps_values_in_report_but_not_stdout_summary() {
    let record_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let report = VaultDecodeReport {
        decoded_value_count: 1,
        values: vec![VaultDecodeValueReport {
            record_id: record_id.to_string(),
            token: "<NAME-1>".to_string(),
            original_value: "Alice Example".to_string(),
        }],
        audit_event: VaultAuditEventReport {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            kind: "decode".to_string(),
            actor: SurfaceKind::Cli,
            detail: "decode event".to_string(),
            recorded_at: "2026-04-29T00:00:00Z".to_string(),
        },
    };

    let report_json = serde_json::to_string(&report).expect("report serializes");
    assert!(report_json.contains("Alice Example"));

    let stdout = build_vault_decode_stdout(PathBuf::from("decode-report.json"), &report);
    let stdout_json = serde_json::to_string(&stdout).expect("stdout serializes");
    assert!(stdout_json.contains("decode-report.json"));
    assert!(stdout_json.contains("decoded_value_count"));
    assert!(!stdout_json.contains("Alice Example"));
}
```

- [x] **Step 6: Run behavior tests to verify RED**

Run: `cargo test -p mdid-cli vault_decode -- --nocapture`

Expected: FAIL because helper/report types and runner are not implemented.

- [x] **Step 7: Implement bounded vault decode runner**

Import `DecodeRequest`:

```rust
use mdid_domain::{
    AuditEvent, AuditEventKind, BatchSummary, DecodeRequest, DicomPrivateTagPolicy, PdfPageRef,
    PdfScanStatus, SurfaceKind,
};
```

Add run dispatch:

```rust
        CliCommand::VaultDecode(args) => run_vault_decode(args),
```

Add report types and helpers near vault audit report types:

```rust
#[derive(Debug, Serialize)]
struct VaultDecodeReport {
    decoded_value_count: usize,
    values: Vec<VaultDecodeValueReport>,
    audit_event: VaultAuditEventReport,
}

#[derive(Debug, Serialize)]
struct VaultDecodeValueReport {
    record_id: String,
    token: String,
    original_value: String,
}

fn parse_record_ids_json(record_ids_json: &str) -> Result<Vec<Uuid>, String> {
    let record_ids: Vec<Uuid> = serde_json::from_str(record_ids_json)
        .map_err(|err| format!("invalid --record-ids-json: {err}"))?;
    if record_ids.is_empty() {
        return Err("decode scope must include at least one record id".to_string());
    }
    Ok(record_ids)
}

fn run_vault_decode(args: VaultDecodeArgs) -> Result<(), String> {
    let record_ids = parse_record_ids_json(&args.record_ids_json)?;
    let request = DecodeRequest::new(
        record_ids,
        args.output_target,
        args.justification,
        SurfaceKind::Cli,
    )
    .map_err(|err| format!("invalid decode request: {err}"))?;
    let mut vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let result = vault
        .decode(request)
        .map_err(|err| format!("failed to decode vault records: {err}"))?;

    let report = VaultDecodeReport {
        decoded_value_count: result.values.len(),
        values: result
            .values
            .into_iter()
            .map(|value| VaultDecodeValueReport {
                record_id: value.record_id.to_string(),
                token: value.token,
                original_value: value.original_value,
            })
            .collect(),
        audit_event: VaultAuditEventReport {
            id: result.audit_event.id.to_string(),
            kind: result.audit_event.kind.as_str().to_string(),
            actor: result.audit_event.actor,
            detail: sanitized_audit_detail(&result.audit_event),
            recorded_at: result.audit_event.recorded_at.to_rfc3339(),
        },
    };

    fs::write(
        &args.report_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to render decode report: {err}"))?,
    )
    .map_err(|err| format!("failed to write decode report: {err}"))?;

    println!(
        "{}",
        serde_json::to_string(&build_vault_decode_stdout(args.report_path, &report))
            .map_err(|err| format!("failed to render decode summary: {err}"))?
    );
    Ok(())
}

fn build_vault_decode_stdout(report_path: PathBuf, report: &VaultDecodeReport) -> serde_json::Value {
    json!({
        "report_path": report_path,
        "decoded_value_count": report.decoded_value_count,
        "audit_event": report.audit_event,
    })
}
```

Update usage string to include:

```text
       mdid-cli vault-decode --vault-path <vault.json> --passphrase <passphrase> --record-ids-json <json-array> --output-target <label> --justification <reason> --report-path <decode-report.json>
```

- [x] **Step 8: Run focused CLI tests to verify GREEN**

Run: `cargo test -p mdid-cli vault_decode -- --nocapture`

Expected: PASS.

- [x] **Step 9: Run broader CLI tests**

Run: `cargo test -p mdid-cli -- --nocapture`

Expected: PASS.

- [x] **Step 10: Commit implementation**

Run:

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-04-29-cli-vault-decode-command.md
git commit -m "feat(cli): add bounded vault decode command"
```

### Task 2: README completion truth-sync for CLI vault decode

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion ledger**

Edit the completion table to raise CLI from `75%` to `78%` and Overall from `62%` to `63%`, with wording that `mdid-cli` now includes bounded `vault-decode` in addition to the existing CSV/XLSX/DICOM/PDF/audit commands.

- [ ] **Step 2: Update capabilities and missing items honestly**

Update the CLI capability bullet to state that `vault-decode` unlocks a local vault, decodes explicit record ids, writes a local JSON report containing decoded values, and prints only a PHI-safe summary. Keep missing items: broader import/export CLI commands and richer workflows remain unimplemented.

- [ ] **Step 3: Verify README mentions no agent/controller roadmap**

Run: `grep -nE "agent|controller|orchestration|planner|coder|reviewer" README.md || true`

Expected: no product-roadmap language for agent/controller/orchestration. Existing negative disclaimers are acceptable only if they explicitly say this product does not implement those behaviors.

- [ ] **Step 4: Run relevant tests after docs change**

Run: `cargo test -p mdid-cli -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit README truth-sync**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync cli vault decode completion"
```

## Self-Review

- Spec coverage: Task 1 implements bounded local CLI decode and Task 2 updates README completion/missing-items status.
- Placeholder scan: No TBD/TODO/fill-in-later instructions remain.
- Type consistency: `VaultDecodeArgs`, `VaultDecodeReport`, `VaultDecodeValueReport`, `parse_record_ids_json`, and `build_vault_decode_stdout` are consistently named across tasks.
