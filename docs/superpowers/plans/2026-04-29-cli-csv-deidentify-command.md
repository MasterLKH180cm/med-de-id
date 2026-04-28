# CLI CSV De-identify Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded, real CLI CSV de-identification command that reads a local CSV, applies explicit field policies through the existing application/vault stack, writes the rewritten CSV, and prints a PHI-safe JSON summary.

**Architecture:** Keep `mdid-cli` thin: parse a small set of local file arguments, deserialize runtime-shaped field policies, unlock or create a local vault, delegate CSV processing to `mdid_application::TabularDeidentificationService`, write only the rewritten CSV to the requested output path, and print a summary/review count envelope without PHI-bearing CSV or review values. This is a de-identification automation surface only; it must not add moat/controller/agent workflow semantics.

**Tech Stack:** Rust, `assert_cmd`, `predicates`, `serde_json`, existing `mdid-application`, `mdid-adapters`, `mdid-domain`, and `mdid-vault` crates.

---

## File Structure

- Modify: `crates/mdid-cli/Cargo.toml`
  - Add direct dependencies on `mdid-adapters` and `mdid-vault` because the CLI constructs field policies and vault stores directly.
  - Add `tempfile` as a dev-dependency for integration tests using isolated local files.
- Modify: `crates/mdid-cli/src/main.rs`
  - Extend command parsing with `deidentify-csv` and exact required flags: `--csv-path`, `--policies-json`, `--vault-path`, `--passphrase`, `--output-path`.
  - Implement policy JSON parsing for array items shaped as `{ "header": string, "phi_type": string, "action": "encode"|"review"|"ignore" }`.
  - Create the vault if the path does not exist, otherwise unlock it, then call `TabularDeidentificationService::default().deidentify_csv(..., SurfaceKind::Cli)`.
  - Write rewritten CSV to `--output-path` and print JSON containing `output_path`, `summary`, and `review_queue_len` only.
  - Keep usage and error output free of moat/controller/agent terms.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add TDD integration tests for a successful CSV run, review-only queue count, malformed policy rejection, and usage text staying de-identification-scoped.
- Modify: `README.md`
  - Truth-sync CLI completion/status to credit the landed CSV automation command and update overall completion only if justified by landed tests.

---

### Task 1: Add bounded CLI CSV de-identification command

**Files:**
- Modify: `crates/mdid-cli/Cargo.toml`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing CLI CSV success test**

Append this test code to `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_deidentify_csv_writes_rewritten_csv_and_phi_safe_summary() {
    let temp = tempfile::tempdir().unwrap();
    let csv_path = temp.path().join("input.csv");
    let vault_path = temp.path().join("vault.json");
    let output_path = temp.path().join("output.csv");
    std::fs::write(&csv_path, "name,notes\nAlice,follow up\nAlice,second visit\n").unwrap();

    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();
    cmd.args([
        "deidentify-csv",
        "--csv-path",
        csv_path.to_str().unwrap(),
        "--policies-json",
        r#"[{"header":"name","phi_type":"name","action":"encode"}]"#,
        "--vault-path",
        vault_path.to_str().unwrap(),
        "--passphrase",
        "correct horse battery staple",
        "--output-path",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"total_rows\":2"))
    .stdout(predicate::str::contains("\"encoded_cells\":2"))
    .stdout(predicate::str::contains("\"review_queue_len\":0"))
    .stdout(predicate::str::contains("Alice").not());

    let rewritten = std::fs::read_to_string(&output_path).unwrap();
    assert!(rewritten.contains("name,notes"));
    assert!(rewritten.contains("tok-"));
    assert!(!rewritten.contains("Alice"));
    assert!(vault_path.exists());
}
```

- [ ] **Step 2: Run the success test to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test cli_smoke cli_deidentify_csv_writes_rewritten_csv_and_phi_safe_summary -- --exact
```

Expected: FAIL because `tempfile` is not yet available and/or `deidentify-csv` is still an unknown command.

- [ ] **Step 3: Write malformed-policy and review-count failing tests**

Append these tests to `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_deidentify_csv_reports_review_queue_count_without_printing_phi() {
    let temp = tempfile::tempdir().unwrap();
    let csv_path = temp.path().join("input.csv");
    let vault_path = temp.path().join("vault.json");
    let output_path = temp.path().join("output.csv");
    std::fs::write(&csv_path, "name,notes\nAlice,patient requests callback\n").unwrap();

    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();
    cmd.args([
        "deidentify-csv",
        "--csv-path",
        csv_path.to_str().unwrap(),
        "--policies-json",
        r#"[{"header":"notes","phi_type":"note","action":"review"}]"#,
        "--vault-path",
        vault_path.to_str().unwrap(),
        "--passphrase",
        "correct horse battery staple",
        "--output-path",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"review_queue_len\":1"))
    .stdout(predicate::str::contains("patient requests callback").not())
    .stdout(predicate::str::contains("Alice").not());

    let rewritten = std::fs::read_to_string(&output_path).unwrap();
    assert!(rewritten.contains("patient requests callback"));
}

#[test]
fn cli_deidentify_csv_rejects_malformed_policy_json_without_scope_drift_terms() {
    let temp = tempfile::tempdir().unwrap();
    let csv_path = temp.path().join("input.csv");
    let vault_path = temp.path().join("vault.json");
    let output_path = temp.path().join("output.csv");
    std::fs::write(&csv_path, "name\nAlice\n").unwrap();

    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();
    cmd.args([
        "deidentify-csv",
        "--csv-path",
        csv_path.to_str().unwrap(),
        "--policies-json",
        r#"[{"header":"name","phi_type":"name","action":"delete"}]"#,
        "--vault-path",
        vault_path.to_str().unwrap(),
        "--passphrase",
        "correct horse battery staple",
        "--output-path",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("invalid policies JSON"))
    .stderr(predicate::str::contains("moat").not())
    .stderr(predicate::str::contains("controller").not())
    .stderr(predicate::str::contains("agent").not());
}
```

- [ ] **Step 4: Run the new tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test cli_smoke cli_deidentify_csv -- --nocapture
```

Expected: FAIL because the command and dependencies are not implemented yet.

- [ ] **Step 5: Add CLI dependencies**

Edit `crates/mdid-cli/Cargo.toml` so the dependencies section contains:

```toml
[dependencies]
mdid-adapters = { path = "../mdid-adapters" }
mdid-application = { path = "../mdid-application" }
mdid-domain = { path = "../mdid-domain" }
mdid-runtime = { path = "../mdid-runtime" }
mdid-vault = { path = "../mdid-vault" }
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true
```

And the dev-dependencies section contains:

```toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
serde_json.workspace = true
tempfile = "3.10"
```

- [ ] **Step 6: Implement minimal CLI command**

Replace `crates/mdid-cli/src/main.rs` with:

```rust
use std::path::PathBuf;
use std::process;

use mdid_adapters::{FieldPolicy, FieldPolicyAction};
use mdid_application::TabularDeidentificationService;
use mdid_domain::SurfaceKind;
use mdid_vault::LocalVaultStore;
use serde::Deserialize;
use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args).and_then(run_command) {
        Ok(Some(output)) => println!("{output}"),
        Ok(None) => {}
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    DeidentifyCsv(DeidentifyCsvArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeidentifyCsvArgs {
    csv_path: PathBuf,
    policies_json: String,
    vault_path: PathBuf,
    passphrase: String,
    output_path: PathBuf,
}

#[derive(Deserialize)]
struct FieldPolicyInput {
    header: String,
    phi_type: String,
    action: FieldPolicyActionInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FieldPolicyActionInput {
    Encode,
    Review,
    Ignore,
}

fn run_command(command: CliCommand) -> Result<Option<String>, String> {
    match command {
        CliCommand::Status => Ok(Some("med-de-id CLI ready".to_string())),
        CliCommand::DeidentifyCsv(args) => run_deidentify_csv(args).map(Some),
    }
}

fn run_deidentify_csv(args: DeidentifyCsvArgs) -> Result<String, String> {
    let csv = std::fs::read_to_string(&args.csv_path)
        .map_err(|error| format!("failed to read CSV input: {error}"))?;
    let policies = parse_policies_json(&args.policies_json)?;
    let mut vault = if args.vault_path.exists() {
        LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
            .map_err(|error| format!("failed to unlock local vault: {error}"))?
    } else {
        LocalVaultStore::create(&args.vault_path, &args.passphrase)
            .map_err(|error| format!("failed to create local vault: {error}"))?
    };

    let output = TabularDeidentificationService::default()
        .deidentify_csv(&csv, &policies, &mut vault, SurfaceKind::Cli)
        .map_err(|error| format!("failed to de-identify CSV: {error}"))?;

    std::fs::write(&args.output_path, output.csv)
        .map_err(|error| format!("failed to write CSV output: {error}"))?;

    Ok(json!({
        "output_path": args.output_path,
        "summary": output.summary,
        "review_queue_len": output.review_queue.len(),
    })
    .to_string())
}

fn parse_policies_json(raw: &str) -> Result<Vec<FieldPolicy>, String> {
    let inputs: Vec<FieldPolicyInput> = serde_json::from_str(raw)
        .map_err(|error| format!("invalid policies JSON: {error}"))?;

    Ok(inputs
        .into_iter()
        .map(|policy| FieldPolicy {
            header: policy.header,
            phi_type: policy.phi_type,
            action: match policy.action {
                FieldPolicyActionInput::Encode => FieldPolicyAction::Encode,
                FieldPolicyActionInput::Review => FieldPolicyAction::Review,
                FieldPolicyActionInput::Ignore => FieldPolicyAction::Ignore,
            },
        })
        .collect())
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [command, rest @ ..] if command == "deidentify-csv" => parse_deidentify_csv_args(rest),
        _ => Err("unknown command".to_string()),
    }
}

fn parse_deidentify_csv_args(args: &[String]) -> Result<CliCommand, String> {
    let mut csv_path = None;
    let mut policies_json = None;
    let mut vault_path = None;
    let mut passphrase = None;
    let mut output_path = None;

    let mut index = 0;
    while index < args.len() {
        let Some(value) = args.get(index + 1) else {
            return Err(format!("missing value for {}", args[index]));
        };
        match args[index].as_str() {
            "--csv-path" => csv_path = Some(PathBuf::from(value)),
            "--policies-json" => policies_json = Some(value.clone()),
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--output-path" => output_path = Some(PathBuf::from(value)),
            flag => return Err(format!("unknown deidentify-csv flag: {flag}")),
        }
        index += 2;
    }

    Ok(CliCommand::DeidentifyCsv(DeidentifyCsvArgs {
        csv_path: csv_path.ok_or("missing --csv-path")?,
        policies_json: policies_json.ok_or("missing --policies-json")?,
        vault_path: vault_path.ok_or("missing --vault-path")?,
        passphrase: passphrase.ok_or("missing --passphrase")?,
        output_path: output_path.ok_or("missing --output-path")?,
    }))
}

fn exit_with_usage(error: &str) -> ! {
    eprintln!("{error}");
    eprintln!();
    eprintln!("{}", usage());
    process::exit(2);
}

fn usage() -> &'static str {
    "Usage: mdid-cli [status|deidentify-csv --csv-path <input.csv> --policies-json <json> --vault-path <vault.json> --passphrase <passphrase> --output-path <output.csv>]\n\nmdid-cli is the local de-identification automation surface.\nCurrent landed commands:\n  status           Print a readiness banner for the local CLI surface.\n  deidentify-csv   De-identify a local CSV with explicit field policies and a local vault."
}
```

- [ ] **Step 7: Run targeted CLI tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test cli_smoke cli_deidentify_csv -- --nocapture
```

Expected: PASS for the `cli_deidentify_csv_*` tests.

- [ ] **Step 8: Run full CLI test target and lint**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test cli_smoke
cargo test -p mdid-cli --all-targets
cargo clippy -p mdid-cli --all-targets -- -D warnings
```

Expected: all commands PASS.

- [ ] **Step 9: Truth-sync README completion snapshot**

Update `README.md` completion rows:

```markdown
| CLI | 55% | Early automation surface with readiness plus a bounded real `deidentify-csv` command that reads local CSV files, applies explicit field policies through the existing application/vault stack, writes rewritten CSV output, and prints PHI-safe summary JSON; active moat/controller CLI behavior has been removed from the product surface rather than counted as completion. |
| Overall | 52% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review/PDF review/DICOM/vault decode/audit/portable export/import entries, browser tabular/PDF review surface with bounded CSV/XLSX/PDF import/export helpers, desktop request-preparation/localhost-submit/response workbench foundation with bounded CSV/XLSX/PDF/DICOM file import/export helpers and bounded vault/portable request helpers, plus a real bounded CLI CSV de-identification command. |
```

Also update the CLI bullet to:

```markdown
- `mdid-cli` remains an early de-identification automation surface. It now includes a bounded `deidentify-csv` command that reads a local CSV, applies explicit field policies via the application/vault stack, writes rewritten CSV output, and prints only PHI-safe summary/review-count JSON. Active moat/controller CLI behavior has been removed from the product surface because agent workflow / controller loop / planner-coder-reviewer coordination semantics are scope drift for med-de-id; future CLI work should expose only de-identification workflows such as additional local tabular runs, vault/decode, audit, import/export, and verification.
```

- [ ] **Step 10: Run README and scope-drift verification**

Run:

```bash
git diff --check
grep -nE 'CLI|Browser/web|Desktop app|Overall|moat|controller|agent|orchestration' README.md
```

Expected: diff check PASS. Grep hits for moat/controller/agent/orchestration are limited to explicit negative scope-drift wording, not product roadmap expansion.

- [ ] **Step 11: Commit the slice**

Run:

```bash
git add crates/mdid-cli/Cargo.toml crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-04-29-cli-csv-deidentify-command.md Cargo.lock
git commit -m "feat(cli): add bounded csv deidentify command"
```

Expected: commit succeeds on `feature/cli-csv-deidentify-command`.

- [ ] **Step 12: Merge verified feature branch into develop**

Run:

```bash
git checkout develop
git merge --no-ff feature/cli-csv-deidentify-command -m "merge: CLI CSV deidentify command"
```

Expected: merge succeeds without conflicts.
