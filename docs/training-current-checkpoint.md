# Single-current native training checkpoint

Cassidy's October 26 decision supersedes earlier recommendations to keep two
rollback checkpoints, milestones, receipt packs or the full training history.
Native serial care should retain one accepted checkpoint. Old launchers remain
legacy readers/reconciliation tools; their history retention is not the policy
for new runs. Use `scripts/run_current_care.py` for the new native serial lane.

`current.alife-checkpoint` contains the complete validated compiler/foundation,
actor coefficients, stage mask, first/second Adam moments and per-coordinate
integer ages, value coefficients/moments/integer age and accepted policy version.
It also embeds the latest admitted private world/organism save, its sleep recovery
journal and every declared asset's original bytes. It has no external episode,
ancestry, correction, compiler or private-asset dependency. The private save is
separate from the ordinary fresh-cohort founder/Adam continuation; resuming a
cohort does not clone a prior individual's personal life into the founder.

Only correctness metadata persists: accepted/source and founder seeds, coalesced
excluded seed ranges, explicitly known/unknown historical freshness and the unused
tail of the current seed allocation. Reserved, consumed, interrupted and assessment
seeds stay excluded from new allocations. The owned unused tail must be consumed
first; its cursor advances durably before dispatch. Migration rejects a tail that
conflicts with declared assessment seeds instead of reclassifying them as usable.
There is no receipt ledger,
historical checkpoint hash chain, lesson score archive or automatic mastery gate.
Disjoint exclusions can still grow if seed allocation is fragmented; contiguous
allocations coalesce. Checkpoint size tracks current state, not episode count.

The Linux synthetic native fixture measured 1,196,536 logical bytes and 1,200,128
inode-allocated bytes for the current checkpoint at each of eight publications,
with one checkpoint file each time. This includes a full N2048 compiler/actor/value
handoff with nonzero Adam moments and a small private world plus a byte asset.
It does not represent PC10641's private GPU tensors or prove PC storage/throughput
savings. Host lifecycle tests separately check two durable store files: current
and the one-byte stable lock. Directory allocation and temporary peak space are
additional; full native CLI publication/relocation is checked on Linux and Windows.
Full-state/private admission adds CPU/I/O work and the final private capture adds
GPU checkpoint work; neither PC cost nor training throughput is measured here.
Encoded checkpoints are bounded at 256 MiB; oversized candidates fail closed
without removing accepted current or the recoverable episode. Run metadata has a
separate 1 MiB bound. Expanded schedules are size-checked before creating `.run`,
with room reserved for bounded attempt bookkeeping; oversized updates preserve
the last readable metadata. Split oversized schedules explicitly rather than
truncating their lessons or silently dropping seed exclusions.

New-format serial episodes capture the final private world at the confirmed
collection boundary. A legacy migration preserves the latest **persisted** private
save and sleep journal, without inventing later state that was never saved. It
must pass the existing native terrain/corrected-critic verifier and private-save
loader before publication. Unsupported or missing legacy state stops migration.

The device-free relocation regression executes the real current-file branch in
`load_cycle_continuation` and compares its complete actor/value state to the
admitted legacy source, including actual corrected critic age. It also compares
compiler bytes and private save/asset bytes after source removal. The advancement
regression checks full heads and rejects wrong optimizer deltas or initial asset.
These tests do **not** execute GPU trainer restoration or the final live-runtime
capture/publish/rebind calls. Their placement before PPO update is source-reviewed
and compile-checked; actual hardware GPU continuation remains for the PC owner's
matching-runtime admission. The native CLI fixture executes no neural decisions,
and host scheduler tests mock the GPU child.

The versioned binary header records JSON payload length and SHA-256; asset bytes
inside the payload use lossless base64, without reducing floating-point precision.
Readers check the complete handoff, native compiler coordinates, private asset
closure and existing save/journal/world contracts. Older binaries do not read this
new format. Original `SOURCE NEW_OUTPUT` portable export and directory-based
native resume retain their existing behavior; conversion is explicit and read-only.

## Publication and interrupted runs

The store holds one stable OS writer lock. It never unlinks that lock file. A new
checkpoint is flushed and admitted, then a flushed publication intent is committed.
The prior current is copied transiently, the candidate atomically replaces current,
and current is reopened and verified before removing the transient prior and intent.
No mutable hardlinks/reflinks or permanent generations are used. Readers see a
complete old or new current during replacement. On restart the intent and exact
hashes reconcile interruption; corrupt/ambiguous state fails closed with surviving
files intact. This tests process interruption; power-loss guarantees also depend
on filesystem/device flush semantics.

All episode worlds, replay, logs, source snapshots, schedule and recovery candidates
are under the owned `.run` directory. Accepted episodes publish current before
their temporary data is removed. Before another run, reconciliation admits any
completed seal exactly once and clears the old owned run. Live/unknown worker PIDs,
unknown store entries, changed pinned executables/helpers or invalid current block
cleanup. No process outside this run is stopped.

Private-save admission uses one lease-protected scratch workspace in the OS temp
directory. Each validation removes its copied world/assets on completion; the
next validation reclaims a killed validator's orphan only after acquiring the
OS lease. It never sweeps arbitrary temporary directories. The empty workspace's
small lease marker persists, without historical private snapshots or a recovery
dependency; copying just current still suffices. Unknown entries or aliases block
cleanup. GPU episode/recovery/rerun artifacts remain under the owned `.run`.

Unsealed interrupted learning is retained automatically. `--reconcile --run`
reports it instead of guessing that a missing seal means no useful work. Recover
and seal it with the original tools when possible. If the owner explicitly chooses
to discard that stopped unsealed attempt, `--reconcile --abandon-unsealed --run`
removes only this store's temporary run after verifying current. Its seed remains
excluded. This option cannot override an active or uncertain writer. It never
removes legacy/production history or replaces accepted current with a reset brain.

## Commands

Build matching native binaries from the admitted revision:

```text
cargo build --release --locked -p alife_game_app --features foundation-training --bin train_n2048_care --bin export_native_training_handoff
```

`export_native_training_handoff` also provides device-free `--inspect-current`,
`--import-current`, `--reserve-current`, `--consume-current`, `--advance-current`
and `--extract-current-life CURRENT NEW_DIRECTORY`. The latter extracts an ordinary
portable world/asset bundle for individual-life recovery; its destination must be
new. Use the host runner for ownership, reservation, publication and cleanup.

Run `python scripts/run_current_care.py --help` for migration/schedule flags. Without
`--run`, mutating modes describe their destination without writing or starting
training. Supply the original seed-history admission policy explicitly at migration.
A new schedule specifies the same existing lesson/decisions format, allocation,
native binary, checkpoint tool, cutoff and resource/idle guards. `--resume --run`
continues an interrupted owned schedule; `--reconcile --run` performs no training.

## PC owner migration and cleanup plan

This cloud change authorizes **no PC deletion**. The PC owner must perform these
steps after matching-binary admission, while training and old launchers are stopped.

1. Recheck the selected accepted source and its actual dependency closure read-only.
   The reported source is PC10641, policy10641, actor/value Adam23165/26281:
   `C:\Users\PC\Documents\Codex\2026-10-07\task-2\care-first-main-f9e3fdfa-20261021\target\founder-training\care-first-relocated-20261023-batch-0067\attempt-00039`.
   Recorded founder SHA-256 is
   `d25aa88b912bb32c11e70fe1163a7dc23ee18aba990a480b569a83fd4d0eef66`.
   Rehash the actual founder, both full checkpoints and selected save/asset closure;
   compare full state, not just optimizer counters. The metadata-only inventory
   reported 35,842,839 bytes/103 files here, not a verified minimal closure.
2. Reconcile the old mixed-care ledger and `care-first-night-20261026/RESERVED-TAIL-HANDOFF.json`.
   Seed202610222681 is abandoned/excluded; 202610222682–202610222688 are reserved
   unused. Verify these against the actual immutable plan/attempts before import.
   Preserve the unfinished schedule cursor and remaining rows as the next explicit
   temporary run plan, including its food-need initialization and deterministic
   prior environment. Do not replace them with defaults. Do not launch expired
   Oct26 cutoffs or reserve another tail.
3. Build/admit the new checkpoint tool and native trainer together. The previous
   runtime is PR160 `f7f7cd5187efda697897c67d9dd7a253702cc2e5`; do not silently swap
   the old launcher chain. It currently runs `care-first-night-20261026/run-night.py`
   → `care-first-security-runtime-20261025/continue-night.py` → `launch-mixed-care.py`
   → `care-first-pr160-prep-20261025/artifact/train_n2048_care.exe`, with the scheduler
   under `care-first-main-f7f7cd51-20261025/scripts/run_mixed_care_serial.py`.
4. Migrate to a **new separate store**, using the exact source/history/lane and its
   strict or explicitly unknown-freshness policy. Inspect current; compare complete
   compiler/actor/value/Adam canonical state and founder digest. Copy just current
   to an unrelated directory; admit it there with old roots unavailable, extract
   and admit its private save/assets/journal, then perform the owner's required
   matched-runtime recovery admission. A failure leaves the original brain/history
   intact and blocks cleanup. No private brains need to leave the PC.
5. Only after that admission, make an exact path manifest of obsolete legacy roots
   and verify no active/unknown writer uses any of them. Propose moving those exact
   paths to a separate owner-approved quarantine/recoverable location; check each
   move outcome before another attempt. Keep the new current store, executables,
   source/assets and unrelated projects out of that manifest. Do not infer a purge
   target from the 28-root size inventory or a broad parent directory.
6. Obtain the required **per-action confirmation** before any permanent purge of
   those exact quarantined paths. No permanent purge is implemented here. Remove
   transient migration/rollback copies only after the verified replacement is
   independently recoverable and the owner has reconciled the move results.

The inventory's 121,440,628,888 logical bytes/284,837 files describe old known roots,
not measured reclaimable allocation. Hardlink allocation, PC migration output size,
private GPU restore and native throughput remain unmeasured by this cloud work.
