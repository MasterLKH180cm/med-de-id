# CLI Vault Audit Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded read-only `mdid-cli vault-audit` command that unlocks a local vault and prints PHI-safe audit metadata as JSON.

**Architecture:** Keep this as a narrow CLI surface on top of the existing `mdid_vault::LocalVaultStore::audit_events()` API. The command must not add controller/moat/agent workflow semantics and must not expose mapping originals or decoded values.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-vault`, `mdid-domain`, serde/serde_json, cargo tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add CLI parsing, usage text, report structs, read-only vault audit execution, and parser/unit tests.
- Modify: `README.md` — truth-sync completion snapshot and implemented/gaps text after the landed CLI command.

### Task 1: Add bounded `vault-audit` CLI command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/src/main.rs` unit tests

- [ ] **Step 1: Write the failing parser test**

Add this test inside the existing `#[cfg(test)] mod tests` block in `crates/mdid-cli/src/main.rs`:

```rust
    #[test]
    fn parses_vault_audit_command_without_requiring_debug() {
        let args = vec![
            "vault-audit".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--limit".to_string(),
            "10".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::VaultAudit(VaultAuditArgs {
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    limit: Some(10),
                }))
        );
    }
```

- [ ] **Step 2: Run parser test to verify RED**

Run: `cargo test -p mdid-cli parses_vault_audit_command_without_requiring_debug -- --nocapture`

Expected: FAIL because `CliCommand::VaultAudit` and `VaultAuditArgs` do not exist.

- [ ] **Step 3: Implement minimal parser support**

In `crates/mdid-cli/src/main.rs`, add:

```rust
    VaultAudit(VaultAuditArgs),
```

to `enum CliCommand`, add this struct near the other args structs:

```rust
#[derive(Clone, PartialEq, Eq)]
struct VaultAuditArgs {
    vault_path: PathBuf,
    passphrase: String,
    limit: Option<usize>,
}
```

Add this match arm in `parse_command` before `_ =>`:

```rust
        [command, rest @ ..] if command == "vault-audit" => {
            parse_vault_audit_args(rest).map(CliCommand::VaultAudit)
        }
```

Add this parser function after `parse_deidentify_pdf_args`:

```rust
fn parse_vault_audit_args(args: &[String]) -> Result<VaultAuditArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut limit = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--limit" => {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| "invalid --limit".to_string())?;
                if parsed == 0 {
                    return Err("invalid --limit".to_string());
                }
                limit = Some(parsed);
            }
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(VaultAuditArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        limit,
    })
}
```

In `run_command`, add:

```rust
        CliCommand::VaultAudit(args) => run_vault_audit(args),
```

Add a temporary stub after `run_deidentify_pdf` so the crate compiles for the parser test:

```rust
fn run_vault_audit(_args: VaultAuditArgs) -> Result<(), String> {
    Err("vault-audit is not implemented".to_string())
}
```

- [ ] **Step 4: Run parser test to verify GREEN**

Run: `cargo test -p mdid-cli parses_vault_audit_command_without_requiring_debug -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Write failing execution test**

Add this test inside `mod tests`:

```rust
    #[test]
    fn vault_audit_report_limits_events_without_exposing_phi_values() {
        let events = vec![
            mdid_domain::AuditEvent {
                id: uuid::Uuid::nil(),
                kind: mdid_domain::AuditEventKind::Encode,
                actor: SurfaceKind::Cli,
                detail: "encoded mapping row:1:patient_name containing Alice Example".to_string(),
                recorded_at: chrono::DateTime::parse_from_rfc3339("2026-04-29T00:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            },
            mdid_domain::AuditEvent {
                id: uuid::Uuid::nil(),
                kind: mdid_domain::AuditEventKind::Decode,
                actor: SurfaceKind::Desktop,
                detail: "decode to screen because break-glass decoded 1 record record_ids=[abc]".to_string(),
                recorded_at: chrono::DateTime::parse_from_rfc3339("2026-04-29T01:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            },
        ];

        let report = build_vault_audit_report(&events, Some(1));
        let rendered = serde_json::to_string(&report).unwrap();

        assert!(rendered.contains("event_count"));
        assert!(rendered.contains("returned_event_count"));
        assert!(rendered.contains("Decode"));
        assert!(!rendered.contains("Alice Example"));
        assert_eq!(report.event_count, 2);
        assert_eq!(report.returned_event_count, 1);
        assert_eq!(report.events[0].detail, "decode to screen because break-glass decoded 1 record record_ids=[abc]");
    }
```

- [ ] **Step 6: Run execution test to verify RED**

Run: `cargo test -p mdid-cli vault_audit_report_limits_events_without_exposing_phi_values -- --nocapture`

Expected: FAIL because `build_vault_audit_report` and report structs do not exist.

- [ ] **Step 7: Implement report builder and real command**

Add `AuditEvent, AuditEventKind` to the `mdid_domain` imports:

```rust
use mdid_domain::{
    AuditEvent, AuditEventKind, BatchSummary, DicomPrivateTagPolicy, PdfPageRef, PdfScanStatus,
    SurfaceKind,
};
```

Replace the temporary `run_vault_audit` stub with:

```rust
#[derive(Debug, Serialize)]
struct VaultAuditReport {
    event_count: usize,
    returned_event_count: usize,
    events: Vec<VaultAuditEventReport>,
}

#[derive(Debug, Serialize)]
struct VaultAuditEventReport {
    id: String,
    kind: AuditEventKind,
    actor: SurfaceKind,
    detail: String,
    recorded_at: String,
}

fn run_vault_audit(args: VaultAuditArgs) -> Result<(), String> {
    let vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let report = build_vault_audit_report(vault.audit_events(), args.limit);
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|err| format!("failed to render audit report: {err}"))?
    );
    Ok(())
}

fn build_vault_audit_report(events: &[AuditEvent], limit: Option<usize>) -> VaultAuditReport {
    let event_count = events.len();
    let selected = events.iter().rev().take(limit.unwrap_or(event_count));
    let events = selected
        .map(|event| VaultAuditEventReport {
            id: event.id.to_string(),
            kind: event.kind.clone(),
            actor: event.actor.clone(),
            detail: sanitized_audit_detail(event),
            recorded_at: event.recorded_at.to_rfc3339(),
        })
        .collect::<Vec<_>>();
    VaultAuditReport {
        event_count,
        returned_event_count: events.len(),
        events,
    }
}

fn sanitized_audit_detail(event: &AuditEvent) -> String {
    match event.kind {
        AuditEventKind::Encode => "encoded mapping".to_string(),
        _ => event.detail.clone(),
    }
}
```

- [ ] **Step 8: Run execution test to verify GREEN**

Run: `cargo test -p mdid-cli vault_audit_report_limits_events_without_exposing_phi_values -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Run all CLI tests**

Run: `cargo test -p mdid-cli -- --nocapture`

Expected: PASS.

- [ ] **Step 10: Run relevant workspace tests**

Run: `cargo test -p mdid-cli -p mdid-vault -p mdid-runtime -- --nocapture`

Expected: PASS.

- [ ] **Step 11: Update usage text**

Extend the `usage()` string to include:

```text
       mdid-cli vault-audit --vault-path <vault.json> --passphrase <passphrase> [--limit <count>]
```

and command description:

```text
  vault-audit         Print bounded PHI-safe vault audit event metadata in reverse chronological order; read-only.
```

- [ ] **Step 12: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs
git commit -m "feat(cli): add bounded vault audit command"
```

### Task 2: README truth-sync for CLI vault audit

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Change the CLI row from 72% to 75%, and overall from 60% to 62%. Add `vault-audit` to the CLI implemented text, and remove audit from the list of remaining CLI command gaps while keeping vault/decode and broader import/export gaps honest.

- [ ] **Step 2: Run README grep verification**

Run: `grep -n "vault-audit\|CLI | 75%\|Overall | 62%" README.md`

Expected: lines showing the new CLI command and updated percentages.

- [ ] **Step 3: Commit README truth-sync**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-cli-vault-audit-command.md
git commit -m "docs: truth-sync cli vault audit completion"
```
