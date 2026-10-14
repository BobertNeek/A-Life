# Current-age sensitive-period repair — 2026-10-02

Implementation prepared on `prototype/sensitive-period`, worktree `/workspace/alife-sensitive-period`, based on `c8c78d28bedcb5910f9f1a1ee30a2601dc6ab180`. Cassidy selected option A: use biological age when learning happens, including sleep replay; recalling childhood does not restore childhood plasticity. This report supersedes the earlier feasibility stop. Architecture references: AOA-DEV-002/003/006 and AOA-PERSIST-001/004. This report records pre-publication CPU evidence. Publication is authorized on the separate `phase3-sensitive-period-current-age` branch; the publication receipt supplies the exact final commit and CI results. No merge, training, GPU execution, or desktop use occurred. Taste branch/worktree and main remain untouched. PredictionResidual and SocialState are excluded.

## Behavior and source changes

The app derives learning age from the authoritative organism record. Newly sealed waking updates use outcome tick because the global world/resident synchronization has not yet advanced. Retained retries use their current retry tick. Sleep uses synchronized current development at staging submission; later consolidation commit promotes already-computed state rather than replaying it again.

- `crates/alife_game_app/src/gpu_live_runtime.rs`: authoritative `learning_development_at` and binding at waking updates/retries. Binding errors enter the existing retained-learning/retry failure path, preserving post-seal memory/topology observation.
- `crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs`: current-age binding in the sleep preamble, within existing staged rollback boundaries.
- `crates/alife_gpu_backend/src/closed_loop_development.rs`: transient `GpuDevelopmentalPlasticity` cache, keyed by phenotype identity, open-period definitions and three inherited rate ceilings. Route factors use either endpoint and maximum overlapping bias, as in the existing compiler.
- `crates/alife_gpu_backend/src/closed_loop_runtime.rs`: organism-owned transient cache and setter; waking submissions capture their own metadata.
- `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`: append metadata after each 44-word outcome and calculate variable row offsets from actual payload size.
- `crates/alife_gpu_backend/src/closed_loop_sleep.rs`: snapshot current metadata and append it after replay samples, without rewriting historical events.
- `crates/alife_gpu_backend/src/closed_loop_buffers/bucket.rs`: reserve bounded payload space using the existing maximum active-tile bound on projection count.
- `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`: shared direct route lookup in immutable plan storage and capped rate helper.
- `crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl`: use current factor for waking learning and normalization.
- `crates/alife_gpu_backend/shaders/closed_loop_replay_learning.wgsl`: use current factor for replay rate after summing event credits.
- `crates/alife_gpu_backend/src/lib.rs`: export the transport module.

The multiplier preserves the existing algebra: inherited rate times `clamp(base_scale * multiplier, 0, 1)`. The runtime equivalent caps scaled rates at their inherited rate ceilings, not at 1. Zero rates remain zero, decoders remain capped, and unrelated routes retain factor 1. The shared immutable receptor table is not mutated, avoiding accidental changes to unrelated routes that share deduplicated receptors.

## Persistence and learned-state ownership

No topology recompilation, phenotype identity change, learned-bank reset, optimizer-state write, persistent schema extension, or GPU readback is introduced. The cache is derived submission metadata, not acquired state. Restore starts with a fresh cache and re-derives current-age factors before learning. A completed sleep checkpoint restores the already-staged result; committing it does not reevaluate old memories with a different factor. A lost submission retried at a later age uses that later age, consistent with option A. The consolidation request digest still identifies cycle input weights/replay, while developmental factors are submission-time context; request-digest-only reproduction of every effective rate is not claimed. Existing job deduplication prevents a second submission from modifying an already-captured payload.

Metadata binding does not write weight, eligibility, activation, replay-journal, or optimizer buffers. The existing waking and replay kernels still own learning mutations. CPU checks can verify transport, immutable phenotype/genome identity and codec reconstruction inputs; actual device-bank preservation and resulting weight deltas remain unmeasured without a GPU run.

## Cost

For R compiled routes, cached metadata is `4 * (4 + R)` bytes, copied into each submitted waking row or sleep payload. Preparing a row also clones this bounded host cache; the patch does not claim zero host allocations. Factors are recomputed only when open-period definitions, phenotype identity, or inherited ceilings change. Ordinary age increments within a period do not rebuild them. The app constructs bounded developmental state at waking/retry updates; sleep reuses synchronized development. There is no CPU per-synapse loop, full readback, history scan, topology rebuild, or GPU buffer allocation added at developmental transitions.

The shader uses direct existing packed-synapse-to-route indexing, constant-time factor/ceiling loads and multiply/min arithmetic for waking learning/normalization or replay. It never searches projections. Frame arenas reserve four plus `max_active_tiles` additional words per waking row and for a sleep payload, consistent with the existing projection-count capacity bound. This increases reserved arena memory; no runtime performance measurement is claimed.

## Tests and independent review

`crates/alife_game_app/src/gpu_live_runtime/sensitive_period_audit.rs` replaces the old red immutable-upload probe with production helper/transport tests. It covers late-world birth, inclusive before/open/close/after boundaries, outcome age before global synchronization, later retry/sleep age, persisted organism-record decoding, fresh-cache reconstruction, and stable phenotype/compiler/genetic upload identity. It uses the built-in N512 foundation's supported sensor profile.

`crates/alife_gpu_backend/tests/developmental_plasticity.rs` uses existing compiler output as the oracle across N512/N1024/N2048 for both route endpoints, maximum overlap, unrelated routes, actual fixed synapses, capped decoders, and unsaturated recurrent boosts for all three rates. It checks current-age cache closure/reconstruction, stable immutable phenotype/genome identity, Naga shader validation, production entrypoint reachability, and immutable route addressing. Unit tests in the transport module verify waking batch row offsets, distinct payloads, preserved prefixes, sleep suffix placement and rejected malformed suffix boundaries.

R2 independent review found and verified fixes for two issues: a route lookup initially used the wrong storage arena, and early binding-error propagation initially bypassed retained-learning handling. The final review found no remaining concrete blocker. No reviewer claimed GPU execution.

The earlier feasibility probe was intentionally red (`0 / 216` eligible recurrent synapses modulated in the immutable upload). It has been removed: the permanent tests now target mutable submission metadata while preserving immutable upload identity.

## Validation and remaining limits

All Cargo work uses `/workspace/cloud-cpu-validation/env.sh` with `CARGO_BUILD_JOBS=1`. CPU builds may compile wgpu dependencies but do not create a GPU adapter or run neural kernels. Final command results are recorded below.

No six-founder, reproduction, biochemical emitter, training, or taste behavior changes are included. GPU causal learning effectiveness, exact device-bank save/restore behavior under the new factors, and measured GPU cost remain unverified. This is CPU-reviewed source evidence, not a GPU qualification or merge claim.

Final CPU results: **293 tests passed; two existing CPU timing probes ignored**. This is 230 app library tests plus 56 backend unit tests and seven backend integration tests. The focused app tests are included in the app total, not counted twice. The strengthened compiler oracle initially exposed inadequate fixed-route fixtures; final fixtures explicitly set inherited zero and 0.5 targeted masks and pass across all three promoted classes. These were fixture corrections, not production behavior changes.

```sh
CARGO_BUILD_JOBS=1 cargo test -p alife_game_app --features gpu-runtime --lib
CARGO_BUILD_JOBS=1 cargo test -p alife_gpu_backend --lib --test developmental_plasticity --test v2_neuromodulatory_wgsl --test eligibility_simulation_time
CARGO_BUILD_JOBS=1 cargo clippy -p alife_gpu_backend -p alife_game_app --features alife_game_app/gpu-runtime --all-targets -- -D warnings
cargo fmt --all -- --check
scripts/check.sh --quick
git diff --check
```

Clippy, formatting, diff checks, core boundaries, and all 77 documentation assertions passed. Logs: `/workspace/sensitive-period-app-tests.log`, `/workspace/sensitive-period-backend-tests.log`, `/workspace/sensitive-period-clippy.log`, `/workspace/sensitive-period-quick.log`. No performance timing probe, training run, GPU adapter, or desktop was used.

Mode: bounded implementation; review class: R2 independent agent. Assumptions: approved current-age policy and existing compiler multiplier/caps. Files changed: source/test/report inventory above. External issues/comments: none created. Remaining limits: actual GPU deltas, device-state restore continuity and measured cost. Risk/rollback: keep this separate from main; the implementation is one isolated change above the taste base. Next action: complete branch CI, then hand the source-bound GPU qualification below to its separate PC owner.

## Source anchors

These relative source links resolve within the exact commit containing this report: [authoritative update age](../../crates/alife_game_app/src/gpu_live_runtime.rs#L565), [waking/retry binding and retained failure handling](../../crates/alife_game_app/src/gpu_live_runtime.rs#L9274), [sleep preamble](../../crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L205), [cached route metadata](../../crates/alife_gpu_backend/src/closed_loop_development.rs#L10), [GPU capped rate helper](../../crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl#L593), [compiler-oracle tests](../../crates/alife_gpu_backend/tests/developmental_plasticity.rs#L39), and [app boundary/restore tests](../../crates/alife_game_app/src/gpu_live_runtime/sensitive_period_audit.rs#L105).

## Bounded PC GPU qualification handoff — recommendations, not executed here

Use the published SHA in an isolated clean checkout on the qualifying PC. Keep the adapter, driver, build profile, seed and logs with every receipt. Run one test thread. These commands perform bounded lifetime-learning checks; do not run trainer binaries, foundation-training commands, or the full population/soak gates as part of this initial qualification.

Existing regression commands (these test surrounding GPU behavior; they do not alone prove dynamic sensitive-period effects):

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity r12_native_success_promotes_exactly_one_transaction -- --exact --nocapture --test-threads=1
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_sleep consolidation_promotes_fast_preserves_genetic_and_commits_once -- --exact --nocapture --test-threads=1
cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore awake_checkpoint_restores_every_mutable_gpu_bank_exactly -- --exact --nocapture --test-threads=1
cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore n512_every_sleep_phase_restores_with_exact_remaining_gpu_work -- --exact --nocapture --test-threads=1
cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore n1024_and_n2048_restore_submitted_lost_jobs_and_completed_staging -- --exact --nocapture --test-threads=1
```

**Required new feature-specific GPU coverage:** the PC qualification owner should add a small GPU-only integration target named `developmental_plasticity_gpu` (not present in this published patch). Reuse [controlled learning fixtures](../../crates/alife_gpu_backend/tests/closed_loop_fast_plasticity.rs#L463) and [exact restore fixtures](../../crates/alife_game_app/tests/gpu_sleep_restore.rs#L265). Run one matched sealed update for each before/open/close/after case in N512/N1024/N2048, not a learning campaign. Hold eligibility, neural activations, chemistry, outcome and initial banks fixed; bind the actual current-age transport. Read device banks through existing test checkpoint APIs and require:

- Measurable targeted unsaturated fast-weight delta matching inherited factor/cap within stated tolerance; unchanged fixed connections, capped decoder response, unrelated routes, genetic weights and non-target state. Use matched control deltas, not merely changed behavior.
- The same childhood replay journal produces current adult replay rates after closure; one bounded sleep transaction per case, with no historical-age boost.
- Save/restore before and after each boundary, pending outcome retry and completed sleep staging preserves all acquired banks/generations and yields the same next update as uninterrupted execution. Completed staging commits once without applying a newly aged factor again. No metadata binding itself may alter banks or any serialized learned-state/moment fields.

After the owner creates that target, its bounded command is:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test developmental_plasticity_gpu -- --nocapture --test-threads=1
```

For initial performance comparison, use the existing single-row runner, not `--backend gpu-closed-loop` with a subset: the latter requires the entire 36-row promotion matrix. Run this exact row on both taste base `c8c78d28bedcb5910f9f1a1ee30a2601dc6ab180` and the published sensitive-period SHA, on the same adapter, with separate result files:

```powershell
cargo run --release -p alife_tools --bin benchmark_tiers -- --single-row --targets configs/gpu_closed_loop_performance_targets_v1.json --class n512 --sensor-profile privileged-affordance-v1 --population 10 --row-output target/sensitive-period-n512-p10.json
```

This row is bounded to 256 warmup and 1,024 measured ticks. Compare inference/plasticity GPU timing, wall time, physical allocation and readback evidence; a completed process with an `Unavailable` row is not qualification. Repeat the row with `--class n1024` and `--class n2048` only if the initial row and correctness checks pass. Attach commit/tree IDs separately because this private row mode does not enforce the full matrix's clean-source provenance. A single row is a regression comparison, not promotion evidence. Also measure the new matched-period GPU fixture with its factor closed versus open; the existing benchmark row alone may not exercise a sensitive-period transition.
