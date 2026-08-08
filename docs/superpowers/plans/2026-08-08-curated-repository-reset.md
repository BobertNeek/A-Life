# Curated Repository Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce `D:\A life` to one worktree, remove regenerable repository debris, and replace obsolete documentation with a small current authority set while preserving user instructions and durable EI0/EI1 results.

**Architecture:** The reset is an ordered destructive workflow. First create an external recovery receipt, then remove merged worktrees, purge generated output, synthesize replacement documentation, delete obsolete documents, and run an independent whole-repository review. Only one mutating worker may operate at a time.

**Tech Stack:** Git worktrees, PowerShell 7, Markdown, existing repository documentation checks.

## Global Constraints

- Keep `D:\A life` as the only worktree.
- Preserve the canonical user-modified `D:\A life\AGENTS.md` byte-for-byte.
- Preserve Git refs, stashes, reflogs, and remote history.
- Preserve `assets\`, `content\`, `models\local\`, committed fixtures, and `crates\alife_tools\reports\`.
- Retain the committed EI0/EI1 reports and EI1 causal sidecar; delete the ignored EI1 raw cache with its merged worktree.
- Do not run Cargo, GPU workloads, or asset generation for cleanup-only tasks.
- Do not mutate the canonical checkout from two worker tasks concurrently.
- The supervisor alone writes the replacement documentation.
- Do not force-push or reset `main`.
- Stop if a noncanonical branch has a commit absent from `main`, a worktree contains a modification beyond `AGENTS.md`, or a retained result hash changes.

---

### Task 1: Create the external recovery receipt

**Files:**
- Create outside Git: `C:\Users\PC\Documents\A life\curated-reset-recovery-2026-08-08\inventory.txt`
- Create outside Git: `C:\Users\PC\Documents\A life\curated-reset-recovery-2026-08-08\result-hashes.txt`
- Create outside Git: `C:\Users\PC\Documents\A life\curated-reset-recovery-2026-08-08\worktree-removal-allowlist.txt`
- Copy outside Git: three noncanonical `AGENTS.md` files and their `.patch` diffs

**Interfaces:**
- Consumes: the five-worktree state at design approval.
- Produces: the recovery directory required by Task 2.

- [ ] **Step 1: Resolve and validate the recovery target**

Run:

```powershell
$recovery = 'C:\Users\PC\Documents\A life\curated-reset-recovery-2026-08-08'
$parent = Resolve-Path -LiteralPath 'C:\Users\PC\Documents\A life'
if (-not $recovery.StartsWith($parent.Path, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Recovery path escaped its approved parent.' }
New-Item -ItemType Directory -Force -Path $recovery | Out-Null
```

Expected: the exact recovery directory exists outside `D:\A life`.

- [ ] **Step 2: Record canonical identity, remote identity, status, worktrees, branches, stashes, and refs**

Run read-only Git commands for `rev-parse`, `status --short`, `worktree list --porcelain`, `branch -vv`, `stash list`, and `show-ref`. Write their complete output to `inventory.txt`.

Expected: the receipt names canonical commit `69edc8ca...` or records and explains a newer upstream-safe commit.

- [ ] **Step 3: Prove every noncanonical tip is merged**

For each of the four exact branch tips, run:

```powershell
git -C 'D:\A life' merge-base --is-ancestor <tip> main
if ($LASTEXITCODE -ne 0) { throw "Unmerged worktree tip: <tip>" }
git -C 'D:\A life' rev-list --count main..<tip>
```

Expected: exit code `0` and unique-commit count `0` for all four.

- [ ] **Step 4: Back up noncanonical instruction edits**

Copy these exact files into separate named subdirectories under the recovery root and write their `git diff -- AGENTS.md` output beside them:

```text
C:\Users\PC\Documents\A life-n2048-m3-integration\AGENTS.md
D:\A life\.worktrees\n2048-foundation-language-lineage\AGENTS.md
D:\A life-brain-gpu-closed-loop\AGENTS.md
```

Expected: three copied files, three patch files, and SHA-256 hashes for every copy.

- [ ] **Step 5: Hash retained results and canonical instructions**

Hash these exact files with SHA-256 and write the output to `result-hashes.txt`:

```text
D:\A life\AGENTS.md
D:\A life\crates\alife_tools\reports\ei0_exit_gate_report.json
D:\A life\crates\alife_tools\reports\ei0_real_fixture_report.json
D:\A life\crates\alife_tools\reports\era1_promotion_report.json
D:\A life\crates\alife_tools\reports\era1_trial_evidence.jsonl.zst
```

Expected: five readable hashes.

- [ ] **Step 6: Write the exact removal allowlist and report**

The allowlist contains only the four worktree roots from Task 2. The worker writes `task-1-report.md` in the SDD workspace with `DONE`, the recovery path, ancestry results, backed-up paths, and hashes.

---

### Task 2: Remove all merged noncanonical worktrees

**Files:**
- Delete exact worktree root: `C:\Users\PC\Documents\A life-n2048-m3-integration`
- Delete exact worktree root: `D:\A life\.worktrees\n2048-foundation-language-lineage`
- Delete exact worktree root: `D:\A life-brain-gpu-closed-loop`
- Delete exact worktree root: `D:\A life\.worktrees\ei1-norn-plus`
- Modify Git metadata through `git worktree remove` and `git worktree prune`

**Interfaces:**
- Consumes: Task 1 recovery receipt and exact allowlist.
- Produces: one registered worktree, `D:\A life`.

- [ ] **Step 1: Recheck process and worktree state**

Confirm no process command line references any removal target. Re-run `git worktree list --porcelain`, `git status --short` in each worktree, and `merge-base --is-ancestor` for every tip.

Expected: no active consumer; only the three known `AGENTS.md` edits; EI1 tracked-clean; all tips merged.

- [ ] **Step 2: Resolve all removal targets before deleting**

Use `Resolve-Path -LiteralPath` for each target. Confirm each resolved path equals one line in Task 1's allowlist and none equals `D:\A life` or a drive root.

Expected: four exact approved roots and no computed or wildcard path.

- [ ] **Step 3: Remove the three backed-up dirty worktrees**

Run sequentially:

```powershell
git -C 'D:\A life' worktree remove --force 'C:\Users\PC\Documents\A life-n2048-m3-integration'
git -C 'D:\A life' worktree remove --force 'D:\A life\.worktrees\n2048-foundation-language-lineage'
git -C 'D:\A life' worktree remove --force 'D:\A life-brain-gpu-closed-loop'
```

Expected: each command exits `0`; no command targets canonical `main`.

- [ ] **Step 4: Remove the merged EI1 worktree and raw cache**

Re-hash the retained `main` report and sidecar, then run:

```powershell
git -C 'D:\A life' worktree remove 'D:\A life\.worktrees\ei1-norn-plus'
```

If Git rejects the removal solely because ignored output remains, confirm tracked status is clean and run the same exact command with `--force`.

Expected: the worktree root, build output, and raw `target\era1-trial-cache\` are removed; the committed report and sidecar remain in `main`.

- [ ] **Step 5: Prune metadata and prove the sole-worktree state**

Run:

```powershell
git -C 'D:\A life' worktree prune --verbose
git -C 'D:\A life' worktree list --porcelain
```

Expected: exactly one `worktree` record, `D:/A life`.

- [ ] **Step 6: Do not delete branches or Git recovery data**

Record remaining branch refs, stashes, and reflogs in the task report. Do not run branch deletion, `git gc`, `git prune`, `git clean`, or reflog expiration.

---

### Task 3: Remove generated and stale test output

**Files:**
- Delete: `D:\A life\target-task4-pure\`
- Delete: `D:\A life\target-task5-review\`
- Delete: `D:\A life\graphify-out\`
- Delete: `D:\A life\.superpowers\brainstorm\`
- Delete: `D:\A life\target\`
- Preserve: `D:\A life\models\local\`, `assets\`, `content\`, committed fixtures, and committed reports

**Interfaces:**
- Consumes: sole-worktree state from Task 2.
- Produces: canonical checkout without regenerable build, test, graph, PID, log, screenshot, or ignored receipt output.

- [ ] **Step 1: Prove targets are generated and inactive**

Confirm every delete target is ignored or untracked, no process command line references it, and no target is a symlink/reparse point escaping `D:\A life`.

Expected: five exact local directories and no active build/GPU process.

- [ ] **Step 2: Snapshot pre-cleanup sizes**

Record each directory's resolved path and byte size in the task report. Do not enumerate source-bound data outside these five paths.

- [ ] **Step 3: Delete only the resolved allowlist**

Use PowerShell `Remove-Item -LiteralPath <exact-path> -Recurse -Force` once per validated target. Do not use wildcards, environment-variable expansion, repository-root deletion, or cross-shell path composition.

Expected: all five targets are absent.

- [ ] **Step 4: Verify preservation**

Confirm these roots still exist:

```text
D:\A life\models\local
D:\A life\assets
D:\A life\content
D:\A life\crates\alife_tools\reports
```

Re-hash the four retained result files and canonical `AGENTS.md` against Task 1.

Expected: hashes match exactly.

- [ ] **Step 5: Report reclaimed bytes and Git state**

Run `git status --short --ignored`, record remaining ignored roots, and write `task-3-report.md`. Do not modify tracked files or commit.

---

### Task 4: Write the replacement documentation

**Files:**
- Modify: `D:\A life\README.md`
- Modify: `D:\A life\docs\AGENTS.md`
- Create: `D:\A life\docs\VISION.md`
- Create: `D:\A life\docs\STATUS.md`
- Create: `D:\A life\docs\ARCHITECTURE.md`
- Create: `D:\A life\docs\ROADMAP.md`
- Create: `D:\A life\docs\DEVELOPMENT.md`
- Create: `D:\A life\docs\EVIDENCE.md`
- Create: `D:\A life\docs\REFERENCE.md`

**Interfaces:**
- Consumes: current source, committed reports, approved design, and completed cleanup receipts.
- Produces: the only surviving user-facing documentation authorities.

- [ ] **Step 1: Build a requirements extraction table**

The supervisor extracts still-current requirements from the controlling spec, ADRs, schooling boundary, future-compatibility notes, active architecture notes, and committed EI0/EI1 reports. Each requirement receives a destination among the eight final documents.

- [ ] **Step 2: Rewrite `README.md` and `docs/AGENTS.md`**

`README.md` becomes the concise project entry point. `docs/AGENTS.md` names the seven `docs/*.md` authorities plus root `README.md`, and removes paths scheduled for deletion.

- [ ] **Step 3: Write vision and current status**

`VISION.md` separates product fantasy, research aspiration, and non-goals. `STATUS.md` distinguishes implemented, integrated, player-visible, and proven state, including the missing GPU-to-voxel bridge and incomplete autonomous lifecycle.

- [ ] **Step 4: Write architecture and reference**

`ARCHITECTURE.md` defines current crates, ownership, authoritative data flow, persistence, GPU/session boundaries, and superseded CPU-fallback policy. `REFERENCE.md` records stable ABI, tiers, language, teacher/SLM, archive, and evidence invariants.

- [ ] **Step 5: Write roadmap, development, and evidence**

`ROADMAP.md` orders work from the production presentation bridge through autonomous lifecycle, truthful controls, EI1 repair, player-loop proof, scale, and release. `DEVELOPMENT.md` contains current Windows commands and gates. `EVIDENCE.md` records source binding, retained artifact paths, EI0's bounded pass, EI1's blocked verdict, and prohibited inference.

- [ ] **Step 6: Run a prose consistency scan**

Search the new documents for contradictory authority, production CPU fallback, completed EI1 promotion, global fixed N2048 assumptions, unchecked placeholders, and references to deletion candidates. Fix every occurrence before Task 5.

---

### Task 5: Delete obsolete documentation and repair references

**Files:**
- Delete obsolete root Markdown and manifest files named in the approved design, except `README.md` and `AGENTS.md`
- Delete: `D:\A life\specs\`
- Delete obsolete contents under `D:\A life\docs\` except `AGENTS.md` and the seven new authority documents
- Delete the current design and plan files after copying their full text into the ignored SDD review workspace
- Modify surviving source/config comments only when required to repair a broken documentation path; do not change behavior

**Interfaces:**
- Consumes: Task 4 replacement documents.
- Produces: final documentation topology with no stale internal references.

- [ ] **Step 1: Copy design and plan into the SDD workspace**

Preserve review copies under this plan's ignored `.superpowers\sdd\...` workspace before deleting tracked execution artifacts.

- [ ] **Step 2: Generate the exact tracked deletion list**

Use `git ls-files` beneath the approved obsolete roots. Exclude root `AGENTS.md`, root `README.md`, `docs\AGENTS.md`, and the seven final `docs\*.md` authorities. Write the exact list to the task report before deletion.

- [ ] **Step 3: Delete the approved obsolete files**

Remove only the exact files and directories generated in Step 2. Do not delete committed reports, source code, assets, models, content, configs, scripts, workflows, fixtures, licenses, or Git metadata.

- [ ] **Step 4: Repair surviving references**

Run focused searches for deleted Markdown basenames and relative paths across tracked text files. Update links or copy still-current requirements into the new authority documents. Behavioral source changes are out of scope.

- [ ] **Step 5: Verify the final documentation topology**

Expected user-facing documents:

```text
README.md
docs/VISION.md
docs/STATUS.md
docs/ARCHITECTURE.md
docs/ROADMAP.md
docs/DEVELOPMENT.md
docs/EVIDENCE.md
docs/REFERENCE.md
```

Operational instruction files `AGENTS.md` and `docs/AGENTS.md` also remain.

---

### Task 6: Independent review, verification, commit, and push

**Files:**
- Review: all tracked changes since commit `69edc8ca7976f830e7b9ed9e5cf69194ba517e30`
- Create ignored review report in the SDD workspace
- Commit only approved tracked changes

**Interfaces:**
- Consumes: Tasks 1–5 and their reports.
- Produces: verified sole-worktree `main` synchronized with `origin/main`.

- [ ] **Step 1: Dispatch a fresh Luna/max whole-reset reviewer**

The reviewer reads the copied design, copied plan, task reports, `git diff --stat`, `git diff --name-status`, full documentation diff, and current repository state. It must return `CLEAN` or concrete `NEEDS_FIXES` findings.

- [ ] **Step 2: Resolve every load-bearing finding through one worker fix wave**

Dispatch one Luna/max fixer for all concrete findings, then one scoped re-review. The supervisor does not silently waive loss of evidence, source, model, fixture, instruction, or current requirement.

- [ ] **Step 3: Run fresh final checks**

Run:

```powershell
git -C 'D:\A life' worktree list --porcelain
git -C 'D:\A life' status --short
git -C 'D:\A life' diff --check
& 'D:\A life\scripts\docs_check.ps1'
```

Also run a relative Markdown-link validator, retained-result hash comparison, forbidden-claim search, final-doc allowlist check, and `git diff --name-only --diff-filter=D` audit for accidental source/asset/model/fixture deletion.

Expected: one worktree, no diff whitespace errors, docs checks pass, links resolve, result hashes match, and deleted tracked files are documentation-only.

- [ ] **Step 4: Stage only intended tracked changes**

Stage the rewritten documentation and approved obsolete-document deletions. Do not stage or alter the pre-existing canonical `AGENTS.md` edit unless the user separately authorizes it.

- [ ] **Step 5: Commit the curated reset**

Commit with a focused message such as:

```text
docs: replace obsolete project documentation
```

Expected: the commit contains documentation changes only. Worktree/junk cleanup remains an untracked filesystem result.

- [ ] **Step 6: Incorporate upstream and push without force**

Fetch `origin`, incorporate upstream non-destructively if it moved, push `main`, fetch again, and compare local `main` with `origin/main`.

Expected: equal commit IDs and no force push.

- [ ] **Step 7: Final receipt**

Report removed worktrees, reclaimed bytes, preserved hashes, final documentation list, review verdict, commits, push result, remaining intentional user dirtiness, and the external recovery path.
