# Offline foundation training

GPU and opt-in CPU backends train the compiled production graph. Training state
is separate from gameplay saves. Native continuation uses
`FoundationTrainingHandoffV1` and retains full actor/critic optimizer state.

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

The next experiment can use two complementary lesson assignments plus a mixed
composition assignment under this same snapshot, retain their private archives,
and perform one central update before fresh-cohort rebind. Compare subsequent
trainer-removed composition and retention outcomes against matched serial mixed
curriculum training with the same starting checkpoint, seeds, lesson doses,
sampling, effective batches and optimizer budget. Collection speed alone is not
evidence of synergy; preservation is a constraint and positive transfer is the
objective. Checkpoint selection and promotion remain outside this boundary.
