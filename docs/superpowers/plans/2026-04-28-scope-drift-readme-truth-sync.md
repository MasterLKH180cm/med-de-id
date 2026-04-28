# Scope Drift README Truth Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Truth-sync README completion/status wording so med-de-id is described as a de-identification product, not an agent/controller platform, while preserving honest current branch status.

**Architecture:** This is a documentation-only stop-loss slice. It updates README status tables and implemented/current-runtime bullets to mark previously landed moat/controller CLI wording as out-of-scope legacy/drift and not part of the product roadmap; no production behavior changes are made.

**Tech Stack:** Markdown documentation, git verification commands.

---

## File Structure

- Modify: `README.md` — update completion snapshot, implemented-so-far bullets, current-runtime paragraph, roadmap wording, and missing-items language.
- Create: `docs/superpowers/plans/2026-04-28-scope-drift-readme-truth-sync.md` — this plan.

### Task 1: README scope-drift stop-loss and completion truth-sync

**Files:**
- Modify: `README.md:62-103`
- Test: documentation verification via `grep` and git diff review; no Rust tests are required because this slice changes documentation only.

- [ ] **Step 1: Verify current problematic README wording**

Run:

```bash
grep -nE 'moat|controller|agent|orchestration' README.md
```

Expected: finds current README lines that mention bounded `moat controller-plan`, `moat controller-step`, or workflow-system wording. These are the target scope-drift documentation items.

- [ ] **Step 2: Update README completion snapshot and missing-items wording**

Replace the `## Current repository status` completion table and surrounding status text with wording that:

```markdown
Completion snapshot, based only on controller-visible landed repository features and verification state:

| Area | Completion | Status |
|---|---:|---|
| CLI | 42% | Early automation surface with local de-identification, vault/decode, audit, and import/export entry points; previously landed moat/controller handoff commands are documented as scope drift and are not counted as product completion |
| Browser/web | 25% | Bounded localhost tabular de-identification page backed by local runtime routes; not a broader browser governance workspace |
| Desktop app | 10% | Early scaffold only; sensitive-workstation review, vault, decode, and audit flows remain mostly unimplemented |
| Overall | 37% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review entry, browser tabular surface, and local CLI foundations are present, but major workflow depth and surface parity remain missing; scope-drift controller/moat CLI wording is not counted as core product progress |

Missing items include deeper policy/detection crates, full review/governance workflows, richer browser UX, desktop app behavior, broader import/export and upload flows, OCR, visual redaction, FCS semantic parsing, media rewrite/export, generalized spreadsheet handling, auth/session handling where needed, production packaging/hardening, and removal or isolation of scope-drift controller/moat CLI surfaces from product-facing documentation and roadmap claims.
```

- [ ] **Step 3: Replace implemented-so-far CLI/moat bullet**

Replace the implemented-so-far bullet that currently says `mdid-cli` has bounded `moat controller-plan` and `moat controller-step` commands with:

```markdown
- `mdid-cli` remains an early automation surface for local de-identification, vault/decode, audit, and bounded import/export operations. Previously landed `moat controller-plan` and `moat controller-step` commands are treated as scope-drift legacy surfaces for product-positioning purposes: they are not part of the med-de-id de-identification roadmap, are not counted toward completion, and should be removed or isolated in a future stop-loss slice rather than expanded
```

- [ ] **Step 4: Narrow current-runtime paragraph and planned-next wording**

Ensure the current runtime paragraph continues to list only de-identification runtime entries. Ensure `Planned next from the design` contains no agent/controller/planner wording and says:

```markdown
Planned next from the design:

- Additional policy and detection crates
- Deeper application behavior and surface behavior beyond the current scaffolds
- Stop-loss cleanup for scope-drift CLI surfaces so product-facing documentation and commands stay aligned with de-identification workflows
```

- [ ] **Step 5: Verify documentation content**

Run:

```bash
grep -nE 'moat|controller|agent|orchestration' README.md
```

Expected: any remaining hits explicitly describe scope drift/legacy stop-loss or occur in non-product wording; there must be no positive roadmap claim that med-de-id is an agent/controller/orchestration platform.

Run:

```bash
git diff -- README.md docs/superpowers/plans/2026-04-28-scope-drift-readme-truth-sync.md
```

Expected: diff is documentation-only and matches this plan.

- [ ] **Step 6: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-28-scope-drift-readme-truth-sync.md
git commit -m "docs: truth-sync readme scope drift status"
```

Expected: commit succeeds on `docs/scope-drift-readme-truth-sync`.

## Self-Review

- Spec coverage: README completion snapshot covers CLI, Browser/web, Desktop app, Overall, missing items, and scope-drift handling.
- Placeholder scan: no TBD/TODO/implement-later placeholders.
- Type consistency: documentation-only slice; no code symbols introduced.
