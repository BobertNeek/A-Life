# Live tick path examination — 2026-10-01

The dominant cause of the roughly 180 ms Windows live tick remains **Unknown**.
The published 95.7% live-tick wall-time share includes GPU completion waits.
It cannot attribute that time to CPU preparation. The existing receipt already
records the internal stages needed to narrow the cause; another profiling
framework is unnecessary.

## Evidence and ownership

Published main `e76e9e123e7dac767a360e9703f365cb1b07d7f4` reports an optimized
Balanced1080p/Vulkan run with eight creatures on RTX 3050: 330 frames in
60.18 seconds, 95.7% simulation tick work, and 6.82 ms average
renderer/present/uninstrumented residual. Its implementation revision is
`c050013d5cefe4760b019e46cb0ebb37d774f300`.
The coordinator additionally reports 1920×1080, i7-3770K/32 GB, 5.48 FPS,
5.50 TPS, and all 330 frames above 100 ms. The
[published island receipt summary](https://github.com/BobertNeek/A-Life/blob/e76e9e123e7dac767a360e9703f365cb1b07d7f4/crates/alife_game_app/assets/landscape/island/README.md#L40)
is available; `target/artifacts/island-hand-implementation/island-hand-performance.json`
and its exact launch command are absent in this cloud checkout. The measured
run mode, save/founder, brain capacity, sensor profile, optional diagnostic
environment, GPU timestamps, and internal stage totals therefore remain Unknown.

This examination owns this report and the isolated CPU probe/evidence under
`docs/performance/evidence`. A subsequent bounded predicate change in
`alife_world/src/headless.rs` is documented separately in the
[terrain sight range receipt](20261001-terrain-sight-range-gate.md). The previous
allocation fix is documented in [its receipt](20261001-live-perception-preparation.md).
Graphics, operational launch source, sandbox save summaries, training budgets,
experimental N512 fixtures, and bundle/shader discovery remain with their owners.
No merge, PR, local task, hardware test, or GPU training campaign was performed.

## Candidate comparison

The examination compares main with CI follow-up
`ad99ac1c31c53d8898703268446a2f785586fe4e` and integration candidate
`f8ccfb13d1c267ac1e83c29504d09f204bc36ffc`. The preparation body in
`gpu_live_runtime/staged_tick.rs`, receptor compiler, canonical digest builder,
GPU learning pipeline, executed-work accounting, graphical scheduler, receipt
writer, and conversation capture UI have identical blobs on all three.
`HeadlessWorld::first_sight_hit` and `BrainPhenotype::recompute_phenotype_hash`
have identical function hashes, although their containing files differ.
[Evidence](evidence/20261001-neural-receptor-preparation.json) retains those
identities. The inference source differs by a monotonic-tick guard; its single
batched submission/completion structure is unchanged.

Candidate commit `312d34f2` already shares immutable phenotype data across exact
**training** snapshots. This report does not propose repeating that repair or
treating training snapshot cost as ordinary live gameplay cost. Current live
cognitive sealing uses compact authority receipts and topology digests, rather
than fetching a full neural snapshot for every awake row
(`gpu_live_runtime.rs:9295`). Ordinary, checkpoint, and sleep-durable readbacks
have separate receipt counters and must be assessed separately.

## Source-supported candidates for the dominant cost

The ordering below is an examination priority, not a measured ranking.
Line references refer to published main.

| Candidate | Source evidence | Existing receipt fields that distinguish it |
| --- | --- | --- |
| GPU inference/learning execution and completion | Inference encodes the cohort, submits once, and waits once across its chunks (`closed_loop_runtime/tick.rs:512`). Learning iterates class/chunk groups (`closed_loop_runtime.rs:4386`); each group submits, polls, and receives compact/timestamp mappings (`closed_loop_pipeline.rs:2840`). A successful single-class, single-chunk awake tick has an inference completion followed by a learning completion. These dependencies preserve selection, physiology, and learning order. | `internal_tick_stages.inference_transaction_ns`, `learning_transaction_ns`; `dispatch_batching`; `readback`; `gpu_stages.timestamp_samples`, `inference_ns`, `plasticity_ns`. |
| Grounded perception and immutable phenotype work | Every dispatchable awake row compiles receptor expression and recomputes the immutable phenotype hash inside the grounded-perception timer (`staged_tick.rs:539–553`, `neural_receptors.rs:18`). For TerrainVision, object visibility and the 16-ray fan call `first_sight_hit`, which scans world objects and every terrain obstacle (`headless.rs:3108–3132`), then marches terrain heights at 0.25-unit spacing. GroundedObjectSlots bypasses these terrain sight rays. Nearby-candidate indexing does not accelerate the obstacle scans. Published island collision uses 1,716 proxies; the exact measured world's obstacle representation and sensor profile are Unknown. | `preparation_substages.grounded_perception_ns`, alongside episodic retrieval, attention, topology, and upload preparation. This timer includes both receptors and world sensing. |
| Transactional rollback copies | Before staged simulation, the runtime clones the authoritative world and resident map (`gpu_live_runtime.rs:240–243`, `7788–7812`). Cost can depend on resident memories and world contents. Removing these copies without replacing rollback would break atomicity. | `transactional_rollback_clone.world_clone_ns`, `residents_clone_ns`, `resident_rows`, `world_object_rows`, progress and zero-progress counts; `simulation.runtime_tick_wall_ns`. |
| Body/world/cognitive validation and persistence | Canonical organism replacement, body/biochemistry sealing, memory compaction checkpoint construction, world advancement, synchronization, and durable sleep/checkpoint activity remain real simulation work. Current genetic digest proofs are already cached with `OnceLock`; repeated genome serialization is not a current hypothesis. | `internal_tick_stages` seal/authority/synchronize/persistence fields; `state_reference_hash`; `ordinary_full_snapshot`; `checkpoint_activity`; `sleep_durable_activity`; `sleep_journal_publication_stages`; zero-progress reasons. |

GPU timestamp coverage must be checked before interpretation. A completed neural
timing sample is published only when inference and successful learning match
dispatch generation, capacity class, and population
(`closed_loop_runtime.rs:4560–4583`). Missing samples do not imply zero GPU work.
Transaction wall time minus covered GPU timestamps includes host preparation,
queue delay, mapping, and validation; it is not an isolated synchronization
measurement. The count named `blocking_transactions` is based on transaction
batch counters and is not a direct count of every device poll.

## Pacing and diagnostics included by recording

`--record-performance` enables the existing detailed preparation timers
(`bevy_shell.rs:1025`) and forces `PresentMode::Immediate`
(`bevy_shell.rs:973`). It retains the chosen simulation run mode. In `1x`, the
graphical scheduler limits planned work to one tick per frame and retains debt
(`bevy_shell.rs:714–735`); it does not sleep inside the live tick. In `max`, an
8 ms frame budget is checked after a completed tick. One tick already taking
about 180 ms therefore also yields roughly one tick per frame in `max`.
Similar FPS/TPS alone does not identify the original run mode.

The recording screenshot sequence opens speech and lineage UI and leaves the
lineage pane open after its last capture
(`production_voxel_renderer.rs:7068–7082`). Speech capture sets an input string;
it does not submit a player utterance. Lineage loading occurs once and is cached
(`production_conversation_lineage_ui.rs:840–890`). Those UI costs are outside
the live neural stage and do not establish the cause of its 95.7% share.

Optional `ALIFE_GRAPHICS_CAPTURE_DIR` screenshots and `ALIFE_ACTION_TRACE_PATH`
JSONL writes run in PostUpdate. They add no neural readback, but can affect frame
wall time and graphics contention. Whether they were enabled in the measured
run is Unknown. The 6.82 ms residual includes renderer, presentation, and
uninstrumented work; it is not an isolated GPU renderer measurement.

## Bounded CPU probe

The [probe](evidence/20261001-neural-receptor-probe.rs) compiles receptor
expression repeatedly from the current built-in GroundedObjectSlotsV1 N512
phenotype: 512 neurons, 1,799 synapses. Three pinned processes each warm 100
calls and collect nine samples of 1,000 eight-call cohorts. The test rlib uses
the existing `alife_core` opt-level 2; the standalone probe uses `rustc -O` on
the same EPYC/KVM cloud host as the earlier allocation receipt.

The median was **0.922470 ms** per eight-call cohort, with a
**0.894752–0.993219 ms** sample range. All samples are retained in
[raw CPU evidence](evidence/20261001-neural-receptor-preparation.json).
This is warmed isolated
CPU work on one phenotype, not eight live residents, the exact Windows save,
or a main/candidate gameplay comparison. Copy-only probe samples exclude
production cache access, invalidation, and fresh chemistry projection and are
not an implemented-cache performance result. Built-in N512 does not support
TerrainVision; no terrain-profile result is claimed.

An eventual private resident-owned `NeuralReceptorPhenotype` cache could retain
compile/hash validation at phenotype admission and replacement, while deriving
effects from fresh chemistry every tick. Birth, restore, reset, and sleep-driven
capacity replacement need explicit coverage. This isolated cost supports that
small opportunity; it does not explain the reported live tick or justify a
broader runtime change before reading the existing dominant-stage totals.

## Smallest next measurement

First retrieve the original JSON and launch transcript. Divide cumulative
stage nanoseconds by `simulation.completed_world_ticks` and frame totals by
the recorded frame count; retain zero-progress counts, timestamp coverage,
sleep transitions, and batch sizes. Check build/source/executable identity.
Rollback cloning is measured outside the staged internal tick body, so do not
assume the internal stage sum covers all runtime tick wall time. The graphical
live stage also includes presentation-frame publication after the runtime tick.

For the independent desktop graphics task, this read-only command extracts the
existing attribution fields without changing the app or running training:

```powershell
$receipt = Get-Content target/artifacts/island-hand-implementation/island-hand-performance.json -Raw | ConvertFrom-Json
$receipt | Select-Object source_head, build, profile, population, resolution, backend, adapter, measurement_seconds, frame, simulation, internal_tick_stages, preparation_substages, gpu_stages, dispatch_batching, transactional_rollback_clone, state_reference_hash, ordinary_full_snapshot, checkpoint_activity, sleep_durable_activity | ConvertTo-Json -Depth 12
```

If that receipt cannot resolve attribution, the minimum independent gameplay
benchmark uses the existing release recorder: five seconds warmup, sixty seconds
measurement, then bounded persistence drain. It is separate from training and
has not been run here. On the original Windows machine, use an isolated copy of
the exact eight-creature save and settings, published implementation source,
Balanced1080p/Vulkan/1920×1080, and the original run mode. If the original mode
cannot be recovered, explicitly label a first run `1x`; do not call it the
original invocation. Example with caller-resolved paths/mode:

```powershell
$env:ALIFE_PHASE31_SOURCE_HEAD = (git rev-parse HEAD).Trim()
Remove-Item Env:ALIFE_GRAPHICS_CAPTURE_DIR, Env:ALIFE_ACTION_TRACE_PATH -ErrorAction SilentlyContinue
.\scripts\run_production_voxel_frontend.ps1 -BuildProfile release `
  -Manifest $BenchmarkManifest -UiSettings $BenchmarkUiSettings `
  -Profile Balanced1080p -Resolution 1920x1080 -GraphicsBackend vulkan `
  -RunMode $MeasuredRunMode -RequireGpu -RecordPerformance
```

Retain the printed command, relevant environment, executable digest, full source
SHA, exact starting save, UI/capture state, and generated
`target/artifacts/phase31-performance/phase31-before-release-population-8.json`.
The command preserves the recorder's built-in screenshot/UI sequence and every
simulation subsystem. If deferred debt or presentation dominates, a second
otherwise identical `max` run can distinguish pacing from runtime cost. It does
not replace the same-mode before/after proof for a source optimization.

The repaired cloud candidates contain other source and graphics differences.
A direct main/candidate FPS comparison would not isolate this allocation patch.
Any later end-to-end comparison needs matching graphics and save inputs prepared
by the integration/graphics owners. No such integration or hardware proof is
claimed by this report.

## Validation

- The standalone CPU probe completed three processes and 27 timed samples.
- Evidence checks verified the probe SHA-256, raw sample summaries, identical
  compared blobs, and identical sight/hash function identities.
- `rustfmt --edition 2021 --check` for the probe and `git diff --check`: passed.
- `bash scripts/docs_check.sh`: 77/77 assertions passed.
- Independent Sol 6.1 source and artifact review (R2): no blocking findings.

This examination does not establish the dominant runtime cause. The unresolved
attribution requires the original stage receipt or the independent gameplay
measurement above. The allocation and subsequent terrain range predicate
changes retain separate CPU tests and evidence; no RTX improvement is claimed.
