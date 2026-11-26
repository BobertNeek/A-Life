# Brain Surgery & Weight-Programming Protocol

This protocol eliminates the drift patterns observed in the 30-day audit (tooling rewrites replacing candidate scoring, mixed weight/engine changes, missing V11 dendritic bytes in backups, 17-round blind edit stacking, synthetic fresh-creature proxy tests, and cloud-reset amnesia).

## Core Invariants (Enforced by Tooling)

1. **Self-Contained Base Bundles (`brain_surgery_eval`)**
   Every base (e.g. `r41`) lives in one SHA-256-verified directory containing:
   - `manifest.json` (`alife.brain_surgery.base_bundle.v1`)
   - `foundation.alife-foundation` (canonical N2048 genetic weights)
   - `dendritic_branches.json` (canonical V11 `DendriticBranchSet`)
   - `scorecard.json` (matched evaluation cases)
   - `reopen_receipt.json` (stamped once via `verify-reopen --stamp-bundle`: proof from a separate OS process that weights **and** V11 branches survive a 1-tick GPU run + save + reload)

   Initializing from a GPU save via `brain_surgery_eval init-base --from-save SAVE.json --asset-root ASSETS` **fails closed** if `exact_cognitive_state.dendritic_branches` is missing. `apply-patch` refuses to overwrite `base` or any existing directory and stages bundles atomically. Read-only `verify-reopen` (without `--stamp-bundle`) never mutates `manifest.json`.

2. **Data-Only Patches (`alife.brain_surgery.patch.v1`)**
   Weight-programming Astras propose JSON patches (`weight_edits` + `dendritic_edits`) pinned to `target_base_id`, `target_foundation_digest`, and `target_dendritic_branches_sha256`.
   - `scripts/surgery_loop.py` serializes updates with an exclusive `.lock` file (`flock`/`msvcrt.locking`) and rejects any proposal while `crates/` has uncommitted engine/gameplay/graphics changes.
   - New engine capabilities stay on a separate git branch and **never** gate candidate comparisons on the active base bundle.

3. **One Unscored Round per Astra, Candidate Digest Binding & Byte-Identical Promotion**
   - An Astra cannot propose a second patch while its previous patch is `unscored`, and cannot stack `R(n+1)` onto an unscored `R(n)`.
   - Materialize winning/promotable candidate bundles with the intended base ID upfront (`--candidate-id r42`) before scoring and stamping (`--stamp-bundle`).
   - `brain_surgery_eval score` forces `deterministic_for_training` semantic priors regardless of ambient `ALIFE_SLM_PRIOR*` variables, runs both base and candidate on the identical scorecard, and ranks by overall weighted average (`failed_cases == 0 && candidate_average > base_average`).
   - `scripts/surgery_loop.py promote` verifies that the candidate bundle's `base_id`, `foundation_digest`, and `dendritic_branches_sha256` match the recorded comparison report **and** stamped `reopen_receipt.json`, then copies the bundle byte-for-byte into backups.

4. **2-Hour Real-Progress Stall Fuse & Cloud Recovery**
   - `last_real_score_utc` advances **only** on a valid matched base-vs-candidate `comparison_report.v1` with zero failed cases. Stock fresh-creature checks and compile checks do **not** advance progress.
   - When `watchdog` enters `ROLLBACK_REQUIRED`, `propose` rejects all new patches (even on an empty queue) until either a matched `record-score` lands or `scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json --recover` verifies read-only reopen on the real `active_base_dir` and records recovery.

## Command Reference

```bash
# 1) Create an authentic self-contained base bundle from a GPU save and stamp its reopen receipt
brain_surgery_eval init-base --output surgery/bases/r41 --base-id r41 --from-save path/to/save.json --asset-root path/to/assets
brain_surgery_eval verify-reopen --bundle surgery/bases/r41 --stamp-bundle

# 2) Initialize the locked state-machine ledger
python3 scripts/surgery_loop.py init --state SURGERY_STATE.json --base-dir surgery/bases/r41 --base-score 92.5 --require-reopen-verified

# 3) Propose, materialize (with target base ID), score against active base, and stamp candidate reopen receipt
python3 scripts/surgery_loop.py propose --state SURGERY_STATE.json --owner astra-1 --patch patches/p1.json
brain_surgery_eval apply-patch --base surgery/bases/r41 --patch patches/p1.json --out surgery/cands/r42 --candidate-id r42
brain_surgery_eval score --base surgery/bases/r41 --candidate surgery/cands/r42 --patch-id p1 --out reports/p1.json
python3 scripts/surgery_loop.py record-score --state SURGERY_STATE.json --patch-id p1 --report reports/p1.json

# 4) Stamp candidate reopen verification and promote byte-identical backup + Drive upload
brain_surgery_eval verify-reopen --bundle surgery/cands/r42 --stamp-bundle
python3 scripts/surgery_loop.py promote --state SURGERY_STATE.json --patch-id p1 --new-base-id r42 \
  --candidate-bundle surgery/cands/r42 --backup-dir surgery/backups

# 5) Check watchdog status / recover cleanly after a cloud reset without altering manifest.json
./scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json --recover
python3 scripts/surgery_loop.py watchdog --state SURGERY_STATE.json --fail-on-stall
```

---

## Standing Prompt for Bob & His 5 Astras (Paste Into Bob's Room / Standing Rules)

```text
STANDING WEIGHT-SURGERY PROTOCOL (MANDATORY — NO EXCEPTIONS)

1. SINGLE SOURCE OF TRUTH: Read `SURGERY_STATE.json` and follow `docs/BRAIN_SURGERY_PROTOCOL.md` before doing anything else. Do not rely on chat memory after any restart.
2. CLOUD GPU PATH: Headless cloud evaluation uses Mesa software Vulkan (`VK_ICD_FILENAMES` + `WGPU_BACKEND=vulkan`) and deterministic prior. Never ask for a physical GPU and never bypass setup with stock fresh-creature proxy tests. Run `./scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json --recover` first after any cloud reset; that runs read-only `verify-reopen` on the REAL active base bundle (without altering `manifest.json`).
3. KEEP WEIGHT SURGERY DATA-ONLY: All 5 Astras doing weight/V11 surgery MUST output `alife.brain_surgery.patch.v1` JSON patches against `active_base_id`, `active_base_foundation_digest`, and `active_base_dendritic_sha256`. Do NOT modify Rust engine/renderer/evaluator code (`crates/`) on the weight-surgery worktree; engine or new-sensor work stays on a separate branch.
4. NO BLIND STACKING: Register every Astra patch with `scripts/surgery_loop.py propose`. Each Astra may have at most 1 `unscored` patch in flight. Materialize each candidate into a fresh directory with its target `--candidate-id`, score it against the active base via `brain_surgery_eval score`, and record it via `scripts/surgery_loop.py record-score` before proposing the next edit.
5. NEW SKILLS GO ON THE SHARED SCORECARD: To score a new gain-of-function skill, add deterministic test case(s) to the scorecard and run BOTH the active base bundle and candidate through the same `brain_surgery_eval score` command. Rank strictly by overall weighted average (`promoted: true` if `candidate_average > base_average` and `failed_cases == 0`).
6. STAMPED REOPEN & BYTE-IDENTICAL BACKUPS ONLY: Promotion requires `promoted: true` AND `brain_surgery_eval verify-reopen --bundle CANDIDATE_DIR --stamp-bundle` passing in a separate OS process before running `scripts/surgery_loop.py promote` (never rename `manifest.json` after stamping). Upload the resulting tarball backup to Google Drive immediately.
7. TWO-HOUR STALL FUSE: Hourly updates and 15-minute cron checks must report `python3 scripts/surgery_loop.py watchdog --state SURGERY_STATE.json --fail-on-stall`. Compile checks, storage checks, and fresh-creature smoke tests count as ZERO progress. If `watchdog` reports `ROLLBACK_REQUIRED`, all new `propose` calls are blocked until you restore `known_good_git_sha` / `active_base_dir` and run `./scripts/bootstrap_cloud_surgery.sh --state SURGERY_STATE.json --recover`.
```
