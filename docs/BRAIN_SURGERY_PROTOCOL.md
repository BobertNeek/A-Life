# Brain Surgery & Weight-Programming Protocol

This protocol eliminates the six drift patterns observed in the 30-day audit (tooling rewrites replacing candidate scoring, mixed weight/engine changes, missing V11 dendritic bytes in backups, 17-round blind edit stacking, synthetic fresh-creature proxy tests, and cloud-reset amnesia).

## Core Invariants (Enforced by Tooling)

1. **Self-Contained Base Bundles (`brain_surgery_eval`)**
   Every base (e.g. `r41`) lives in one SHA-256-verified directory containing:
   - `manifest.json` (`alife.brain_surgery.base_bundle.v1`)
   - `foundation.alife-foundation` (canonical N2048 genetic weights)
   - `dendritic_branches.json` (canonical V11 `DendriticBranchSet`)
   - `scorecard.json` (matched evaluation cases)
   - `reopen_receipt.json` (proof from a separate OS process that weights **and** V11 branches survive a 1-tick GPU run + save + reopen)

   Initializing from a GPU save via `brain_surgery_eval init-base --from-save SAVE.json --asset-root ASSETS` **fails closed** if `exact_cognitive_state.dendritic_branches` is missing. A base cannot be promoted or treated as backed up unless `reopen_receipt.json` is verified.

2. **Data-Only Patches (`alife.brain_surgery.patch.v1`)**
   Weight-programming Astras propose JSON patches (`weight_edits` + `dendritic_edits`) pinned to `target_base_id`, `target_foundation_digest`, and `target_dendritic_branches_sha256`.
   - `scripts/surgery_loop.py propose` rejects any patch while `crates/` has uncommitted engine/gameplay/graphics changes.
   - New engine capabilities (new sensors, looped lobes, terrain/UI fixes) belong on a separate git branch and **never** gate candidate comparisons on the active base bundle.

3. **One Unscored Round per Astra, Matched Average Promotion**
   - An Astra cannot propose a second patch while its previous patch is `unscored`, and cannot stack `R(n+1)` onto an unscored `R(n)`.
   - `brain_surgery_eval score` runs both the active base bundle and the candidate bundle on the identical scorecard (`--scorecard` can add new skill tests run on **both** base and candidate).
   - Promotion checks overall weighted average (`candidate_average > base_average`), noting any single-case regressions without vetoing a clear net win, and rejecting if `failed_cases > 0`.

4. **2-Hour Real-Progress Stall Fuse & Cloud Recovery**
   - `scripts/surgery_loop.py record-score` advances `last_real_score_utc` **only** on a valid matched base-vs-candidate `comparison_report.v1` with zero failed cases. Stock fresh-creature checks and compile checks do **not** reset the clock.
   - On any cloud reset, run `scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json`. It automatically configures Mesa software Vulkan (`VK_ICD_FILENAMES` / `WGPU_BACKEND=vulkan`), builds `brain_surgery_eval`, and runs `verify-reopen` on the **real active base**.

## Command Reference

```bash
# 1) Create an authentic self-contained base bundle from a GPU save (never drops V11 branches)
brain_surgery_eval init-base --output surgery/bases/r41 --base-id r41 --from-save path/to/save.json --asset-root path/to/assets

# 2) Verify the real base survives 1 GPU tick + save + separate-process reopen
brain_surgery_eval verify-reopen --bundle surgery/bases/r41

# 3) Initialize the state-machine ledger
python3 scripts/surgery_loop.py init --state SURGERY_STATE.json --base-dir surgery/bases/r41 --base-score 92.5

# 4) Register one Astra's data-only patch, materialize candidate bundle, score against base, and verify reopen
python3 scripts/surgery_loop.py propose --state SURGERY_STATE.json --owner astra-1 --patch patches/p1.json
brain_surgery_eval apply-patch --base surgery/bases/r41 --patch patches/p1.json --out surgery/cands/p1
brain_surgery_eval score --base surgery/bases/r41 --candidate surgery/cands/p1 --patch-id p1 --out reports/p1.json
python3 scripts/surgery_loop.py record-score --state SURGERY_STATE.json --patch-id p1 --report reports/p1.json

# 5) Promote winner after separate-process reopen verification and copy full bundle to backup + Drive
brain_surgery_eval verify-reopen --bundle surgery/cands/p1
python3 scripts/surgery_loop.py promote --state SURGERY_STATE.json --patch-id p1 --new-base-id r42 \
  --candidate-bundle surgery/cands/p1 --reopen-receipt surgery/cands/p1/reopen_receipt.json --backup-dir surgery/backups

# 6) Check watchdog status / recover after a cloud reset
scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json
python3 scripts/surgery_loop.py watchdog --state SURGERY_STATE.json
```

---

## Standing Prompt for Bob & His 5 Astras (Paste Into Bob's Room / Standing Rules)

```text
STANDING WEIGHT-SURGERY PROTOCOL (MANDATORY — NO EXCEPTIONS)

1. SINGLE SOURCE OF TRUTH: Read `SURGERY_STATE.json` and follow `docs/BRAIN_SURGERY_PROTOCOL.md` before doing anything else. Do not rely on chat memory after any restart.
2. CLOUD GPU PATH: Headless cloud evaluation uses Mesa software Vulkan (`VK_ICD_FILENAMES` + `WGPU_BACKEND=vulkan`) and deterministic prior (`ALIFE_SLM_PRIOR_BACKEND=deterministic`). Never ask for a physical GPU and never bypass setup by running stock fresh-creature proxy tests. Run `scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json` first after any cloud reset; that tests `verify-reopen` on the REAL active base bundle (weights + V11 branches).
3. KEEP WEIGHT SURGERY DATA-ONLY: All 5 Astras doing weight/V11 surgery MUST output `alife.brain_surgery.patch.v1` JSON patches against `active_base_id`, `active_base_foundation_digest`, and `active_base_dendritic_sha256`. Do NOT modify Rust engine/renderer/evaluator code (`crates/`) on the weight-surgery worktree; engine or new-sensor work stays on a separate branch and never blocks candidate scoring.
4. NO BLIND STACKING: Register every Astra patch with `scripts/surgery_loop.py propose`. Each Astra may have at most 1 `unscored` patch in flight. Never stack `R(n+1)` onto an unscored `R(n)`. Score every patch against the active base via `brain_surgery_eval score` and record it via `scripts/surgery_loop.py record-score` before proposing the next edit.
5. NEW SKILLS GO ON THE SHARED SCORECARD: To score a new gain-of-function skill, add deterministic test case(s) to the scorecard and run BOTH the active base bundle and candidate through the same `brain_surgery_eval score` command. Rank strictly by overall weighted average (`promoted: true` if `candidate_average > base_average` and `failed_cases == 0`; small regressions are noted, not automatic vetoes).
6. COMPLETE BACKUPS ONLY: Promotion requires BOTH `promoted: true` AND `brain_surgery_eval verify-reopen` passing in a separate OS process (`reopen_receipt.json` proves weights + V11 `dendritic_branches.json` survived 1 live tick, save, and reload). Run `scripts/surgery_loop.py promote` to snapshot the complete bundle and upload the tarball backup to Google Drive immediately.
7. TWO-HOUR STALL FUSE: Hourly updates and 15-minute cron checks must report `python3 scripts/surgery_loop.py watchdog --state SURGERY_STATE.json`. Compile checks, storage checks, and fresh-creature smoke tests count as ZERO progress. If `watchdog` reports `ROLLBACK_REQUIRED` (`>= 2.0h` with no matched base-vs-candidate score), STOP debugging tools, check out `known_good_git_sha`, restore `active_base_dir` from backup/Drive, and run `scripts/bootstrap_cloud_surgery.sh`.
```
