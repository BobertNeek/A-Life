# Offline foundation training

GPU and opt-in CPU backends train the compiled production graph. Training state
is separate from gameplay saves. Native continuation uses
`FoundationTrainingHandoffV1` and retains full actor/critic optimizer state.

The [CPU PPO cost ledger](../../docs/performance/20261031-cpu-ppo-costs.md) records
synthetic component timings, tape reuse, full-state checks and unmeasured costs.

## Cooperative collector boundary (CPU V1)

With `cpu-backend`, independent canonical CPU hosts can collect under one frozen
founder and submit `CpuCollectorShardV1` evidence to one central learner:

1. Capture `CpuCollectionSnapshotV1` from the exact trainer, value head, compiler
   inputs, PPO config and next collection policy version. It binds full actor and
   critic checkpoints (weights, moments, ages, masks/config), compiler inputs,
   sampling temperature, discount/GAE settings and biological-v2/critic-v1
   objective versions.
2. Allocate distinct `CpuCollectorAssignmentV1` run IDs and explicit world/seed,
   producer revision, lesson and episode-config digest. The caller supplies this
   approved inventory again at admission. Counter ranges cover every collected
   decision, including living bootstrap observations. Retain each host's native
   checkpoint and private archive separately.
3. Seal each complete assignment's native `CpuCollectedDecision` trajectories
   and final `WorldOrganismRecord`. `serde_json` round-trips the envelope; no new
   neural trajectory codec or collector-owned optimizer is introduced.
4. Call `admit_cpu_collector_shards`. It validates all seals, snapshot and planned
   assignment identities, unique runs/lives and complete sampling counters;
   reproduces native action sampling and old probabilities; checks captured
   features including bootstrap and rebuilds biological credit. It then calls
   `cpu_training_windows` to rebuild full per-life GAE and residual critic targets
   before slicing detached burn-in windows. `(collector_run_id, organism_id)`
   accompanies every window; local organism IDs may repeat across runs.
5. Consume `AdmittedCpuCollectorCohort::train`. It rechecks the exact central
   snapshot, then uses `train_cpu_residual_cohort` and its frozen preflight. There
   is one canonical actor Adam history and one canonical critic Adam history,
   sample-weighted across each effective batch. Partial updates still need the
   ordinary pre-cohort checkpoint rollback if execution fails.

The `collector_shards` tests in `cpu_collector` exercise two independent synthetic
hosts sharing one snapshot, wire round-trip, colliding local organism IDs, live
bootstrap versus terminal boundaries, per-life GAE and one central actor/critic
step. Rejection cases check unchanged optimizer checkpoints. These are untrained
fixtures, not evidence of learned competence or positive transfer. Run them with:

```sh
cargo test --locked -p alife_training --features cpu-backend --test cpu_collector collector_shards
```

This first boundary is in memory, CPU-only and homogeneous n2048. It does not
schedule collectors, stream large shards, authenticate a hostile producer,
reconstruct a world from a declared seed, admit GPU/mixed-backend evidence, or
hot-swap genetics into retained private lifetime state. Dead records must seal
biology at their death tick. SHA-256 seals detect changed evidence; caller-owned
assignment metadata supplies provenance, not cryptographic attestation. Existing
continuation CLIs still use their serial collection path. Older care diagnostic
rows remain readable with an absent sampling counter; cooperative admission
rejects that absence instead of fabricating sampling evidence.

## Collect-only extraction

`collect_cpu_episode(config, &mut hooks)` now shares native source admission and
collection with `run_cpu_continuation`. The source is an explicit `previous`
directory or a development fixture. The returned `CpuCollectedEpisode` exposes
read-only actor, critic, compiler and private host checkpoints. No offline actor
or critic update runs; ordinary waking plasticity, memory, biology and sleep
remain active in the private host. Serial continuation still prepares its replay
windows, finishes the same episode hooks, and invokes its existing learner.

Consume `collected.seal(approved_assignment)` to publish the existing
`CpuCollectorShardV1` plus `collector-commit.json`. Publication synchronizes and
pins the private checkpoint, collection policy, compiler inputs, frozen optimizer
checkpoints, source ancestry and hook-produced artifacts before acknowledging the
commit. `read_cpu_collector_output(directory)` validates the exact inventory and
every digest before returning a shard for ordinary learner admission. Pending,
torn or changed outputs fail closed; files remain available for explicit
inspection/reconciliation. `phase.txt` is informational. The separate
`collection-metadata.json` records zero offline updates; care hook fields named
`trained_*` describe rows eligible for subsequent loss, not a completed update.

This is a library boundary, not a concurrent care CLI. Existing `CareEpisode`
hooks retain their serial whole-episode seed reservation and mutable live prior
cache behavior. A future adapter must address both before concurrent dispatch.
Collection outputs contain no trained continuation receipt and cannot become a
native `previous` source or advance an accepted lineage. Automatic collection
resume and cooperative continuation/promotion remain unsupported. On hook
failure, the last valid native host checkpoint is retained when checkpoint
publication succeeds; an incomplete collection never gets a usable shard seal.

The next experiment can use two complementary lesson assignments plus a mixed
composition assignment under this same snapshot, retain their private archives,
and perform one central update before fresh-cohort rebind. Compare subsequent
trainer-removed composition and retention outcomes against matched serial mixed
curriculum training with the same starting checkpoint, seeds, lesson doses,
sampling, effective batches and optimizer budget. Collection speed alone is not
evidence of synergy; preservation is a constraint and positive transfer is the
objective. Checkpoint selection and promotion remain outside this boundary.

## Bounded cooperative CPU round

`run_cpu_cooperative_round` joins already sealed collector directories to the
existing central learner. The caller supplies an exact native CPU
`FoundationTrainingHandoffV1` file, a sealed `CpuCooperativeRoundPlanV1`, an
explicit run-ID-to-directory inventory and an isolated output directory. The plan
pins the raw source handoff and collector-commit bytes, exact frozen snapshot,
assignments (including seed/counter/lesson provenance), PPO settings, replay
window/burn-in and effective batch size. Membership is canonical by run ID;
admission also orders trajectories by organism ID before forming batches.

All source, manifest, private recovery and replay evidence is admitted before
learning. Inputs are checked again before updates and publication. The round
restores locally owned actor/critic optimizers, invokes
`AdmittedCpuCollectorCohort::train` once, and exports/rebinds the existing handoff
format for a fresh future cohort. No private lifetime host receives new genetics.
The round is limited to 64 collectors, 65,536 decisions/windows and 256 MiB of
aggregate shard bytes; the source handoff is limited to 128 MiB. PPO epochs and
effective batch size remain explicit plan inputs, with at most 65,536 paired
optimizer updates and no integer-step overflow.

`cooperative-round.json` is a self-contained result receipt containing the plan,
source and resulting full optimizer handoffs, central update receipt, loss-row
count and composite window identities. Its validation checks paired global-step
deltas, exact trainable ages, frozen weights/moments/ages, stage/config and
compiler continuity. A KL stop before any update retains the exact source
handoff and its old policy version; it does not invent a completed new policy.

Publication reuses the collect-only exclusive durable file operations. A retry
validates and re-synchronizes a completed own-plan stage or canonical result
without learning again. Torn own-plan stages remain intact, and a fresh attempt
can recompute from the immutable source. Foreign files, different plans,
tampered complete stages and contradictory completed results fail closed.
`read_cpu_cooperative_round` recovers the committed handoffs without needing the
external collector directories; running/retrying the coordinator still requires
the exact complete input inventory. The caller must exclusively own the output;
concurrent coordinator writers are unsupported.

This remains a library operation with synthetic correctness tests. It does not
dispatch concurrent care episodes, isolate the shared live prior cache, change
whole-episode seed reservations, provide native cooperative continuation
admission or promote any result. Those adapters and a trainer-removed mixed-skill
assessment are still needed for an actual synergy comparison against matched
serial mixed-curriculum learning.

## Disposable connection-budget replay experiment

```sh
cargo run --locked -p alife_training --features cpu-backend --bin experiment_connection_budget -- --disposable-public-founder --seed 70411 --ticks 32
```

This opt-in prototype creates a fresh public built-in N2048 founder, collects
canonical CPU world decisions, ranks recurrent connections using only the first
half's past coactivity, and compares that selection with a seeded control on the
second half's actual baseline frames. The JSON stdout receipt reports per-lobe
requested and resolved connection counts, capacity constraints, source/frame/
effective-state/topology identities, candidate-logit changes and single-pass
CPU timings. It verifies that the founder checkpoint and repeated baseline
replay remain identical. It writes no checkpoint or native handoff and accepts
no existing individual or trained-source path.

The 0/16/32/48/64 settings are **average recurrent fan-in budgets per lobe**:
`requested = fan_in * lobe_neurons`. Each is explicitly capped to the available
compiled recurrent pool; the receipt flags every constraint. Degrees need not
be uniform across neurons. The library's `exact_plan` instead requires a feasible
explicit connection count for every enabled lobe and rejects missing, duplicate,
unknown or excessive budgets. Decoders and dendrites retain their original
state. Existing route gates and cadence still determine which selected
connections fire at each microstep.

This is an effective connection-mask experiment through the real CPU replay
engine. Inactive connections have their detached effective weights cancelled;
adaptive selection can reactivate entries in that same compiled pool. All world
actions remain baseline actions, and each replay starts from the actual baseline
state. It does not execute a closed-loop experimental creature, create absent
coordinates, train weights, or demonstrate learning. Zeroed entries are still
scanned, and trial timings include cloning and hashing; they cannot demonstrate
a throughput gain. Native topology admission and continuation guards remain
unchanged. The next step for a full adaptive-wiring trial is an explicit fresh
experimental phenotype/compiler contract with rebuilt coordinate-bound learning
plans, followed by compact native execution and matched closed-loop trials.

```sh
cargo test --locked -p alife_training --features cpu-backend --lib connection_budget_experiment
```

The separate compact-evaluation integration adds a source-weight-bound,
forward-only index view without changing native topology or training semantics:

```sh
cargo run --release --locked -p alife_training --features cpu-backend --bin experiment_connection_budget -- --disposable-public-founder --seed 70511 --ticks 32 --compact-eval
```

It compares complete output bits for 0/16/full masks on held-out canonical-world
frames. Seven alternating pairs include reference source validation/hash/clone/
cancellation or compact-view construction/binding/destruction plus forward.
Returned outputs are destroyed afterward. Neither path controls experimental
world actions. The receipt reports collection and internal work intervals;
measure external process wall separately to include startup and final output.
[Raw results and identities](../../docs/performance/20261103-budget-compact-eval.json)
record two 32-tick CLI runs (1.230/1.226 s external wall). Pair ratios of roughly
3.38/2.48/2.17 for zero/16/full selection include substantial preparation savings;
the full-selection result is evidence against attributing all savings to pruning.
These are detached forward-replay results, not whole-training or capability gains.

Plans bind phenotype, selected indices and source weights. A compact view checks
every source weight bit on evaluation. This permits precise phenotype-compatible
counterfactual replay; it does not claim ownership of an input's original world.
There is no compact-view gradient, optimizer, checkpoint-export or accepted-source
entry point. Source changes require a fresh plan/view. Native baseline state and
all decoder/dendritic behavior are preserved.

The [native-reference follow-up](../../docs/performance/20261103-full-mask-native-reference.json)
corrects the interpretation of the full-mask ratio. On 16 real captured rows,
seven warm rotating release trials measured native Reference 7.420 ms, native
CadenceCsr 6.726 ms, cancellation 17.826 ms, fresh compact 8.185 ms and reused
compact 7.302 ms (medians). Fresh compact is 10.3% slower than genuine native
Reference. Repeated full-plan SHA/source binding measured 9.347 ms per batch;
sequence cloning 0.198 ms; one native sequence-validation pass 0.367 ms; raw
native graph forward 7.025 ms. These isolated component calls overlap broader
scopes and must not be summed as an exact decomposition. External wall for the
whole assay, including collection, all alternatives, controls and component work,
was 1.182 s. The older direct-lookup main binary was not separately timed.

No new full-mask preparation/training API follows from this result: native replay
already avoids the cancellation wrapper and retains its graph. Existing opt-in
`CpuReplayExecution::CadenceCsr` is reusable through genetic Adam updates; a new
real-frame test verifies complete output/gradient/checkpoint equivalence, with
nonzero moments and integer ages above 2^24. The frozen-weight detached view
correctly rejects those updates and remains forward-only. All new measurement
helpers are test-only; defaults, native checkpoints and accepted brains remain
unchanged.

```sh
cargo test --release --locked -p alife_training --features cpu-backend --lib connection_budget_experiment::full_mask_assay::full_mask_components_and_native_reference -- --ignored --exact --nocapture --test-threads=1
```
