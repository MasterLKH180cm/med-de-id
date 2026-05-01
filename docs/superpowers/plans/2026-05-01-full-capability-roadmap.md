# Full Capability Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Drive med-de-id from the current bounded foundations to full local-first medical de-identification capability across CLI, runtime, browser, desktop, and de-identification workflow/controller surfaces.

**Architecture:** Keep de-identification semantics in shared Rust crates and keep each surface thin: CLI for automation, runtime for localhost contracts, browser for local workflow composition/run control, desktop for sensitive workstation operation. Newly re-authorized AI agent/orchestration/controller capability is limited to de-identification workflow coordination: jobs, queues, review handoffs, audit trails, retry/resume, and controlled local execution; it is explicitly not a general-purpose agent platform.

**Tech Stack:** Rust workspace (`mdid-domain`, `mdid-vault`, `mdid-adapters`, `mdid-application`, `mdid-runtime`, `mdid-cli`, `mdid-browser`, `mdid-desktop`), existing Python OCR/Privacy Filter research runners where explicitly wrapped, serde JSON contracts, Axum runtime tests, egui/browser state tests, strict TDD/SDD.

---

## Current truth-sync baseline — 2026-05-01

- Branch creating this roadmap: `feature/full-capability-roadmap-cron-2251` from `develop`.
- Current README snapshot before this roadmap: CLI 96%, Browser/Web 99%, Desktop app 99%, Overall 97%.
- Open GitHub issues: `#4` DICOM, `#5` PDF/OCR, `#6` conservative media/FCS remain open; `#1`-`#3` are closed.
- Current gap cluster from README and plans: full OCR, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, richer browser/desktop workflow depth, production packaging/hardening, broader controller/workflow orchestration.
- Because no full-capability roadmap existed, this cron round is allowed to land the roadmap/tracking foundation before selecting the next code task. Future rounds must use this roadmap to select implementation work, not keep producing plan-only churn.

## Non-goals and product boundaries

- Do not build SaaS/cloud-first processing, unrestricted plugin execution, or a general-purpose agent platform.
- Do not claim model-quality OCR/handwriting/visual-redaction completion from synthetic fixtures alone.
- Do not expose raw PHI in stdout, browser panes, desktop panes, report filenames, debug output, audit details, controller handoff packets, or runtime error envelopes.
- Do not treat review-only analysis as rewrite/export completion.
- Do not expand browser/desktop completion percentages unless verified UI/runtime/user-flow capability lands.

## Capability acceptance matrix

| Capability | CLI | Runtime | Browser | Desktop | Controller/orchestration | Acceptance gate |
|---|---|---|---|---|---|---|
| Text-only PII detection/masking POC | Existing baseline plus deeper detectors | Existing `/privacy-filter/text` and summaries | Must run or import summaries safely | Must run or import summaries safely | Job step can route text inputs and summaries | Real tests for categories, PHI-safe output, offline/no-network metadata |
| OCR | Existing PP-OCRv5 handoff wrappers | Local OCR execution endpoint or job step | Submit local image/PDF page OCR job or review summary | Submit local image/PDF page OCR job or review summary | Job controller can schedule OCR stage and persist safe evidence | Non-mock local runtime-path proof plus truthful model-quality status |
| Visual redaction | Not complete | Adapter/application capability with bounding boxes/regions | Review redaction regions and blocked pages | Review redaction regions and blocked pages | Queue review tasks for visual redaction decisions | Tests prove raw pixels/text not leaked and unsupported cases fail closed |
| Image pixel redaction | Not complete | Rewrite image pixels from approved regions | Save redacted image artifacts | Save redacted image artifacts | Track rewrite/export artifacts and audit | Pixel-diff tests and PHI-safe artifact metadata |
| Handwriting recognition | Not complete | Explicit handwriting candidate route/status | Review handwriting-needed status | Review handwriting-needed status | Route manual review when recognition unsupported/low confidence | Tests prove honest unsupported/low-confidence handling |
| Final PDF rewrite/export | Not complete | Rewrite/export PDFs with text/region redactions | Download rewritten PDF | Save rewritten PDF | Job controller coordinates OCR/review/rewrite/export | Valid PDF output tests; no review-only overclaim |
| Browser/desktop UI capability | Strong helpers, more depth needed | Existing localhost contracts | File import/run/save flows | File picker/run/save flows | Run-control/status integration | UI/state tests plus build/smoke evidence where possible |
| De-id workflow controller/orchestration | Legacy moat surfaces need product scoping | Job/run APIs needed | Run-control UX needed | Workstation queue UX needed | Local-only de-id jobs/review/audit/handoff | No arbitrary agent platform semantics; all commands de-id scoped |

---

## Task 1: Roadmap and completion-rubric truth-sync (this first round)

**Files:**
- Create: `docs/superpowers/plans/2026-05-01-full-capability-roadmap.md`
- Modify: `README.md`

- [x] **Step 1: Verify no existing full-capability roadmap exists**

Run:

```bash
rg --files docs/superpowers/plans | grep full-capability-roadmap || true
```

Expected: no existing file.

- [x] **Step 2: Create this roadmap with capability tasks, non-goals, acceptance gates, test commands, and parity matrix**

Expected: this document exists and covers text-only PII detection/masking POC, OCR, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, browser/desktop UI capability, and de-id workflow controller/orchestration.

- [x] **Step 3: Truth-sync README without changing completion percentages**

Expected: README records that the roadmap landed, percentages remain unchanged because no product code capability landed.

- [ ] **Step 4: Verification**

Run:

```bash
git diff --check
grep -n "full-capability roadmap\|CLI | 96%\|Browser/Web | 99%\|Desktop app | 99%\|Overall | 97%" README.md
grep -n "text-only PII\|OCR\|visual redaction\|image pixel redaction\|handwriting\|final PDF rewrite\|controller/orchestration" docs/superpowers/plans/2026-05-01-full-capability-roadmap.md
```

Expected: all commands pass. No Rust tests are required for this docs-only tracking foundation.

## Task 2: Text-only PII detection/masking parity hardening

**Files:**
- Test: `scripts/privacy_filter/test_run_privacy_filter.py`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`

- [ ] **Step 1: Add RED tests for the next unsupported PII class**

Add one bounded detector at a time. Suggested next class: phone extension / alternate phone punctuation, because current README lists phone baseline but coverage is not yet complete enough for production claims.

Run targeted RED commands:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerDetectionTests::test_detects_phone_extensions_without_leaking_raw_values -q
source "$HOME/.cargo/env" && cargo test -p mdid-cli privacy_filter_text_detects_phone_extensions_without_raw_phone_leaks --test cli_smoke -- --nocapture
```

Expected RED: missing test or category/count mismatch.

- [ ] **Step 2: Implement minimal detector and CLI/runtime category acceptance**

Keep previews redacted, keep `network_api_called: false`, and bound adjacent-token false positives.

- [ ] **Step 3: Surface parity**

Add browser/desktop tests proving imported/runtime summaries preserve safe phone category counts and omit raw phone strings.

- [ ] **Step 4: Verification**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
source "$HOME/.cargo/env" && cargo test -p mdid-cli privacy_filter --test cli_smoke
source "$HOME/.cargo/env" && cargo test -p mdid-runtime privacy_filter -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser privacy_filter -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop privacy_filter -- --nocapture
git diff --check
```

Acceptance: all outputs remain PHI-safe; README may bump only if a real requirement is added/completed.

## Task 3: OCR execution depth and model-quality evidence

**Files:**
- Test: `tests/test_ocr_runner_contract.py`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: `scripts/ocr_eval/run_small_ocr.py`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`

- [ ] **Step 1: Add RED contract tests for non-mock local OCR execution evidence**

Use deterministic local adapter/fake binary/module tests before requiring real external model weights. Prove stdin/argv/path PHI safety and `engine_status` honesty.

Run:

```bash
python3 -m pytest tests/test_ocr_runner_contract.py::test_local_paddleocr_execution_records_model_quality_unverified_without_source_leak -q
```

- [ ] **Step 2: Add runtime OCR execution or job-step endpoint**

Endpoint must accept bounded base64 image/page input, never expose raw text in summary response unless explicitly writing a sensitive handoff artifact, and classify dependency/model-quality failures honestly.

- [ ] **Step 3: Browser/desktop UI handoff**

Add UI request helpers for OCR run/review summary, not full visual redaction. Tests must verify raw OCR text is not displayed in safe summary panes.

- [ ] **Step 4: Verification**

Run:

```bash
python3 -m pytest tests/test_ocr_runner_contract.py tests/test_ocr_handoff_contract.py -q
source "$HOME/.cargo/env" && cargo test -p mdid-cli ocr --test cli_smoke -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-runtime ocr -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser ocr -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop ocr -- --nocapture
git diff --check
```

Acceptance: local runtime-path proof lands; model-quality status is truthful; no visual-redaction or PDF-rewrite claim yet.

## Task 4: Visual redaction and image pixel redaction foundation

**Files:**
- Test: `crates/mdid-domain/tests/visual_redaction_models.rs`
- Test: `crates/mdid-adapters/tests/image_redaction_adapter.rs`
- Test: `crates/mdid-application/tests/visual_redaction.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Create/Modify: `crates/mdid-domain/src/lib.rs`
- Create/Modify: `crates/mdid-adapters/src/image_redaction.rs`
- Modify: `crates/mdid-application/src/lib.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`

- [ ] **Step 1: Add RED domain tests for redaction-region models**

Model must carry page/image ref, bounded rectangles, decision/status, confidence, and redacted Debug output.

- [ ] **Step 2: Add RED adapter tests for pixel rewrite**

Use a tiny synthetic image with a known red pixel/black box region; assert pixels in approved region change and pixels outside remain unchanged. Test malformed/out-of-bounds regions fail closed.

- [ ] **Step 3: Runtime/application surface**

Add only bounded image-region redaction first. Do not claim face detection or automatic object detection unless implemented/tested.

- [ ] **Step 4: Browser/desktop review/save helpers**

Render region counts and safe artifact names; never render source filenames or raw image bytes.

- [ ] **Step 5: Verification**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-domain visual_redaction -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-adapters image_redaction -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-application visual_redaction -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-runtime visual_redaction -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser visual_redaction -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop visual_redaction -- --nocapture
git diff --check
```

Acceptance: image pixel redaction foundation is verified; automatic visual detection remains non-goal unless a later task adds it.

## Task 5: Handwriting recognition workflow honesty

**Files:**
- Test: `crates/mdid-domain/tests/handwriting_workflow_models.rs`
- Test: `crates/mdid-adapters/tests/pdf_adapter.rs`
- Test: `crates/mdid-application/tests/pdf_deidentification.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: shared PDF/OCR models and surface renderers
- Modify: `README.md`

- [ ] **Step 1: Add RED tests for handwriting-needed status**

A page/image with handwriting suspicion must route to review/manual handling unless a real recognizer is installed and explicitly verified.

- [ ] **Step 2: Implement status propagation**

Add stable wire status such as `handwriting_review_required`, with redacted Debug and PHI-safe report output.

- [ ] **Step 3: Surface parity**

Browser and desktop must show clear next-step copy: handwriting recognition is pending/manual unless verified by a later local recognizer task.

- [ ] **Step 4: Verification**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-domain handwriting -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-adapters pdf_adapter -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-application pdf_deidentification -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-runtime handwriting -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser handwriting -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop handwriting -- --nocapture
```

Acceptance: no false claim that handwriting OCR is complete; review routing is explicit and test-covered.

## Task 6: Final PDF rewrite/export

**Files:**
- Test: `crates/mdid-domain/tests/pdf_workflow_models.rs`
- Test: `crates/mdid-adapters/tests/pdf_rewrite_adapter.rs`
- Test: `crates/mdid-application/tests/pdf_deidentification.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify/Create PDF rewrite adapter/application/runtime/CLI/browser/desktop files
- Modify: `README.md`

- [ ] **Step 1: Add RED tests for valid rewritten PDF bytes**

Test text-layer redaction first. The output must parse as PDF and exclude original PHI strings in extractable text.

- [ ] **Step 2: Add application and vault-backed replacement semantics**

Approved text candidates use vault-backed tokens; review-needed pages remain blocked or marked partial according to explicit policy.

- [ ] **Step 3: CLI/runtime/browser/desktop export paths**

CLI writes PDF to `--output-path`; runtime returns base64 bytes; browser downloads; desktop saves. All summaries are PHI-safe.

- [ ] **Step 4: Verification**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-adapters pdf_rewrite -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-application pdf_deidentification -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-runtime pdf -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-cli deidentify_pdf --test cli_smoke -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop pdf -- --nocapture
source "$HOME/.cargo/env" && cargo test --workspace
```

Acceptance: final PDF rewrite/export may be credited only after valid rewritten artifacts pass parsing and PHI-leak tests.

## Task 7: Browser/desktop end-to-end UI capability depth

**Files:**
- Test/Modify: `crates/mdid-browser/src/app.rs`
- Test/Modify: `crates/mdid-desktop/src/lib.rs`
- Test/Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Add RED tests for file import/run/save state machine**

Cover CSV/XLSX/PDF/DICOM/media/privacy/OCR/vault/portable modes with path/source redaction and no stale output after errors.

- [ ] **Step 2: Wire richer run-control and save affordances**

Keep runtime localhost-only; do not add SaaS/auth/session unless a later explicit task needs it.

- [ ] **Step 3: Build/smoke evidence**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-browser --lib
source "$HOME/.cargo/env" && cargo check -p mdid-browser
source "$HOME/.cargo/env" && cargo test -p mdid-desktop --all-targets
source "$HOME/.cargo/env" && cargo clippy -p mdid-desktop -p mdid-browser --all-targets -- -D warnings
```

If a runnable UI smoke or screenshot is feasible in the environment, capture it and label it as real UI evidence. If not feasible, document the exact blocker.

Acceptance: UI completion moves only after verified user-flow depth lands.

## Task 8: De-identification workflow controller/orchestration platform

**Files:**
- Test: `crates/mdid-domain/tests/job_workflow_models.rs`
- Test: `crates/mdid-application/tests/job_controller.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify/Create: job/workflow models, application controller, runtime routes, CLI commands, browser/desktop run-control surfaces
- Modify: `README.md`

- [ ] **Step 1: Add RED domain tests for bounded de-id job model**

Model stages: ingest, extract, detect, review, encode, export, decode, audit. Include status, retry count, safe handoff summary, actor/surface, artifact refs, and audit linkage. Debug must redact PHI and local paths.

- [ ] **Step 2: Add local job controller service**

Controller can create local jobs, advance deterministic stages, pause for review queue, resume after decisions, and emit audit events. It must not launch arbitrary agents, execute arbitrary shell commands, or generalize beyond de-id workflows.

- [ ] **Step 3: Add CLI/runtime/browser/desktop controls**

CLI commands such as `mdid-cli workflow run/status/resume` are local-only and de-id scoped. Runtime routes mirror job create/status/resume. Browser/desktop show run status, review queue counts, and safe handoff artifacts.

- [ ] **Step 4: Verification**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-domain job_workflow -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-application job_controller -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-runtime workflow -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-cli workflow --test cli_smoke -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser workflow -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop workflow -- --nocapture
source "$HOME/.cargo/env" && cargo clippy --workspace --all-targets -- -D warnings
```

Acceptance: de-id workflow orchestration is product-scoped and auditable; no generic planner/agent platform claims.

## Task 9: Mainline convergence, issues, and release evidence

**Files:**
- Modify: `README.md`
- Modify: this roadmap if truth-sync reveals changed scope
- GitHub issue comments/closures when authenticated or token fallback is available

- [ ] **Step 1: After each verified feature branch, merge to develop**

Run:

```bash
git checkout develop
git merge --no-ff <feature-branch> -m "merge: <slice summary>"
source "$HOME/.cargo/env" && cargo test --workspace
git push origin develop
```

- [ ] **Step 2: Update GitHub issue status**

Use `gh` when authenticated; otherwise use the token in `~/.git-credentials` without printing it.

- [ ] **Step 3: Promote to main only after releaseable verification**

Run divergence checks and use a permitted worktree if `main` is occupied elsewhere. Do not force-push.

- [ ] **Step 4: Final release gate**

Run:

```bash
source "$HOME/.cargo/env" && cargo test --workspace
source "$HOME/.cargo/env" && cargo clippy --workspace --all-targets -- -D warnings
git diff --check
git rev-list --left-right --count origin/develop...origin/main
```

Acceptance: develop/main convergence is truthfully reported; nonzero right-side merge commits are not misreported as missing product work.

---

## Next recommended implementation order

1. Complete the open `2026-04-30-core-completion-safety-disclosures.md` medium tasks that README still lists as open.
2. Add one real code slice from Task 2 or Task 8 per cron run, using strict TDD and SDD review.
3. Only after core disclosure gaps close, tackle OCR execution depth, visual/pixel redaction, handwriting, and final PDF rewrite/export.
4. Keep README and this roadmap truth-synced every round with branch, commit, tests, and unchanged/changed percentages.
