# Curated Repository Reset Design

**Date:** 2026-08-08  
**Repository:** `D:\A life`  
**Canonical branch:** `main`  
**Approved direction:** Keep one canonical worktree, retain durable EI1 results, delete merged worktrees and regenerable output, then replace obsolete documentation with a small current set.

## Goal

Reduce the repository to one worktree and one clear documentation authority without losing merged source, current user instructions, durable evaluation results, source assets, models, or recovery data.

## Starting state

- Canonical checkout: `D:\A life`
- Canonical commit: `69edc8ca7976f830e7b9ed9e5cf69194ba517e30`
- `origin/main` matches the canonical commit.
- Five registered worktrees exist.
- Every noncanonical branch has zero commits that are absent from `main`.
- The canonical checkout has a user-modified `AGENTS.md` and two untracked generated Cargo target directories.
- Three noncanonical worktrees have only an uncommitted `AGENTS.md` edit.
- The EI1 worktree is tracked-clean and points at the same commit as `main`.

## Hard constraints

1. Keep `D:\A life` as the only worktree.
2. Do not replace, normalize, or discard the canonical `D:\A life\AGENTS.md` edit.
3. Preserve the exact contents of each noncanonical `AGENTS.md` before removing its worktree.
4. Preserve the committed EI0 and EI1 result artifacts in `main`.
5. Preserve Git metadata, refs, stashes, reflogs, and the remote.
6. Delete no source asset, model, fixture, or committed report merely because it is large.
7. Treat ignored build products and raw trial caches as regenerable unless this specification names them as durable results.
8. Rewrite documentation only after the worktree and generated-output state is stable.
9. Stage and commit only the intended documentation and tracked cleanup changes.
10. Never force-push or destructively reset `main`.

## Durable result boundary

The retained EI1 result is the committed, source-bound evaluation package:

- `crates/alife_tools/reports/era1_promotion_report.json`
- `crates/alife_tools/reports/era1_trial_evidence.jsonl.zst`
- the producing source commit and tree recorded inside the report
- the adapter/backend identity and honest `Blocked` verdict recorded inside the report

The following committed EI0 results also remain:

- `crates/alife_tools/reports/ei0_exit_gate_report.json`
- `crates/alife_tools/reports/ei0_real_fixture_report.json`

The ignored EI1 raw cache under `target\era1-trial-cache\` is not part of the retained repository result. It contains regenerable per-trial intermediates for already-committed aggregate and causal evidence. It will be deleted with the merged EI1 worktree after the committed report and sidecar are hash-verified in `main`.

## Phase 1: Recovery receipt

Before deletion, an executor creates a temporary recovery directory outside the repository under:

`C:\Users\PC\Documents\A life\curated-reset-recovery-2026-08-08\`

It contains:

- the exact four-worktree inventory
- branch and commit identities
- ancestry results showing zero unique commits
- copies of the three noncanonical modified `AGENTS.md` files
- diffs of those files against their respective `HEAD` versions
- SHA-256 hashes for the four committed EI0/EI1 result files
- a list of removed worktree roots and generated-output roots

This directory is a temporary rollback aid. It is not copied into the repository and is not a substitute for committed documentation.

## Phase 2: Worktree consolidation

Remove these registered worktrees after the recovery receipt exists:

1. `C:\Users\PC\Documents\A life-n2048-m3-integration`
2. `D:\A life\.worktrees\n2048-foundation-language-lineage`
3. `D:\A life-brain-gpu-closed-loop`
4. `D:\A life\.worktrees\ei1-norn-plus`

The first three require forced worktree removal only because their backed-up `AGENTS.md` files are modified. EI1 requires no force if Git remains clean; its ignored cache is intentionally deleted with the worktree.

After removal:

- prune stale worktree metadata
- verify `git worktree list --porcelain` reports only `D:\A life`
- retain branch refs until final verification is complete
- do not prune Git objects, reflogs, or stashes

## Phase 3: Generated-output cleanup

Delete regenerable output from the canonical checkout:

- `target-task4-pure\`
- `target-task5-review\`
- `graphify-out\`
- `.superpowers\brainstorm\`
- Cargo build products under `target\debug\` and `target\tmp\`
- stale PID and log files under `target\` whose processes no longer exist
- obsolete ignored screenshots, review receipts, and temporary fixtures under `target\artifacts\` when they are not a named committed result

Preserve:

- `models\local\`
- `assets\`
- `content\`
- committed fixtures
- `crates\alife_tools\reports\`
- any ignored output that a current source file or verification script explicitly requires and cannot regenerate

The junk executor must resolve each final path before deletion and must not use a broad recursive command against the repository root.

## Phase 4: Documentation reset

Git history is the historical archive. Obsolete plans do not remain in the working tree solely for archaeology.

The final documentation topology is:

- `README.md` — entry point, prerequisites, supported launch modes, and links
- `docs/VISION.md` — product fantasy, research aspiration, and non-goals
- `docs/STATUS.md` — current implemented/integrated/player-visible/proven state and known gaps
- `docs/ARCHITECTURE.md` — current module map, authority boundaries, and production data flow
- `docs/ROADMAP.md` — ordered work beginning with the GPU-to-voxel bridge and autonomous lifecycle
- `docs/DEVELOPMENT.md` — Windows setup, build, focused tests, GPU gates, docs checks, and repository hygiene
- `docs/EVIDENCE.md` — receipt rules, source binding, EI0 result, EI1 blocked result, and scale boundaries
- `docs/REFERENCE.md` — stable ABI, brain tiers, teacher/SLM boundary, persistence, and archive rules

`docs/AGENTS.md` remains as a subtree instruction file. It is operational agent guidance, not part of the user-facing documentation count, and is updated to name the new authorities.

`docs/architecture_decisions.md` is rewritten as a concise current ADR register or replaced by an equivalent section in `docs/ARCHITECTURE.md`. Superseded ADR prose does not remain active without an explicit historical label.

The reset removes:

- obsolete duplicate root specifications
- `SPEC_PACK_MANIFEST.md`
- stale root handoff and research copies superseded by the new docs
- `specs\`
- `docs/codex_plan_pack\`
- `docs/codex_progress\`
- `docs/playable_sim_spec\`
- completed historical productization and Superpowers plan packs
- obsolete archived documentation that has no evidentiary role beyond Git history
- duplicate or superseded architecture notes after their still-current requirements are incorporated

The reset preserves committed machine-readable result artifacts. It does not preserve every historical plan, completion note, screenshot narrative, or handoff prompt.

## Documentation content requirements

The rewritten documentation must state these facts consistently:

- Production neural execution is GPU-authoritative WGSL.
- The world remains authoritative for candidate legality and outcomes.
- The GPU live runtime is real, but production voxel presentation does not yet consume its authoritative world state.
- The main simulation tick is not yet a complete autonomous organism lifecycle.
- EI0 passed only its bounded exit contract.
- EI1 executed its source-bound corpus and remains honestly `Blocked`.
- EI2 and larger brain tiers remain locked.
- CPU neural helpers are baselines, tests, or developer tools, not production fallback.
- `Standard2048` is a reference tier rather than a global fixed shape.
- The external teacher acts through perception; the internal SLM remains a private semantic prior.
- Historical FVR and CPU-shadow receipts are not current production-authority evidence.

## Executor responsibilities

Luna/max workers perform:

- recovery receipt creation
- worktree removal and metadata pruning
- generated-output cleanup
- internal-link and stale-reference scans
- independent post-cleanup review

The supervisor performs:

- all documentation synthesis and rewriting
- final conflict resolution between old authorities
- staging decisions
- final verification, commit, upstream reconciliation, push, and handoff

No two workers may mutate the canonical checkout concurrently.

## Failure handling

- If any noncanonical branch gains a unique commit, stop before removal.
- If a worktree contains a modification beyond `AGENTS.md`, stop and report the exact path.
- If the committed EI1 report or sidecar differs from `HEAD`, stop before deleting EI1.
- If a cleanup candidate is consumed by current source or a verification script, preserve it until its replacement is explicit.
- If documentation still links to a removed file, update the link or restore the required content before committing.
- If upstream moves, incorporate it non-destructively before pushing.

## Verification

Completion requires fresh evidence for all of the following:

1. `git worktree list --porcelain` reports exactly one worktree.
2. `git status --short` contains no accidental deletions or generated output.
3. The canonical user-modified `AGENTS.md` content remains intact.
4. All four committed EI0/EI1 result files exist and match their pre-cleanup hashes.
5. No tracked source, asset, model, or fixture was deleted by generated-output cleanup.
6. Every relative Markdown link in the surviving docs resolves.
7. Searches find no surviving documentation claim of production CPU fallback or completed EI1 promotion.
8. Repository documentation checks pass.
9. The final documentation set is internally consistent with current source.
10. The cleanup and documentation changes are committed intentionally.
11. `main` is pushed without force.
12. Local `main` equals `origin/main` after the push.

## Explicit exclusions

- No production Rust, WGSL, asset, or gameplay behavior changes.
- No Cargo or GPU workload is needed merely to remove worktrees and rewrite documentation.
- No EI1 rerun, promotion, cache relabeling, or larger-brain authorization.
- No branch-ref deletion unless separately authorized after the sole-worktree state is verified.
- No removal of the external temporary recovery receipt during this reset.
