# Explicit new individual with retained training heads

`export_native_training_handoff --begin-new-life` is an opt-in offline training
transition from an observed-terminal, exact same-individual N2048 Current. The
source individual remains terminal. This is an experimental learned-policy clone
into a new individual, not genetic reproduction or resurrection.

The transition imports the exact compiler inputs, topology and actor/value
checkpoints. It preserves every actor weight, Adam moment, per-coordinate age,
mask and configuration; every critic coefficient/moment and its global Adam age;
and policy/objective versions. It performs no optimizer update or world tick.
The new body uses ordinary canonical newborn physiology and fresh private neural,
fast/lifetime, memory, predictor, sleep and sampler state. Source private memories
and somatic deltas are archived with the dead source; they are not transferred or
claimed as memories of the new individual. New-life PPO row/update counters start
at zero independently of inherited optimizer ages.

The ordinary Current schema remains version 1, with an optional, versioned
`new_life_origin` field, omitted from older checkpoints. It records the exact source
file SHA-256, terminal observation and original baseline/request hashes, previous
attempt provenance, imported-head hashes, new-body identity and conservative seed
history. The normal same-individual path remains responsible for later updates.
Old executables cannot read a checkpoint carrying this new field; keep all writers
on the qualified source commit. Existing checkpoints keep their original encoding.

The output is create-new and isolated from the source:

- `terminal-source.alife-checkpoint`: exact source bytes, including old private state/death.
- `new-life-baseline.alife-checkpoint`: immutable initial new-body Current.
- `world.json`, `world-assets/`, `lineage/`: native birth construction/recovery evidence.
- `store/current.alife-checkpoint`: the sole working Current for the ordinary runner.

Never replace the terminal source or original baseline with this child. Retain the
whole output and existing source/baseline archives. A failed creation leaves its
partial output for inspection; it cannot be retried into that output. Incompatible
canonical body/genome/compiler identity, missing assets, changed source bytes,
reused IDs/seeds and incomplete/tampered source checkpoints are rejected. Acquired structural topology outside the global optimizer coordinates is also rejected rather than silently dropped.

## Owner qualification and continuation

Run only in the PC owner's admitted resource window, with live training stopped.
Use a clean source checkout of the published commit, not another experimental
brain/compiler branch. Do not upload private checkpoints or their outputs.

```powershell
cargo test -p alife_game_app --features cpu-training --lib new_life -- --nocapture
cargo test -p alife_game_app --features "cpu-training,gpu-tests" --lib new_life_gpu_initialization_and_continuous_recovery -- --ignored --nocapture
cargo build --release -p alife_game_app --features foundation-training --bin export_native_training_handoff --bin train_n2048_care
```

The second test uses only disposable public/synthetic state on the actual native
GPU, then exercises birth, full native admission, actual continuous-world PPO and
recovery. It is ignored by ordinary CPU tests; an ignored result is not a GPU pass.

Select a new organism ID greater than all retained lineage IDs. Select two
consecutive unused owned seed entries: the new world seed and world seed + 1 for
its initialization boundary. If a pending allocation exists, these must be its
first two entries; its remaining tail is retained. With no pending allocation,
the constructor allocates exactly these two entries. All consumed/reserved prior
ranges remain excluded. Historical `unknown` remains `unknown`; the operation
cannot establish global freshness. The sole lineage owner must serialize child
creation and run only one chosen branch. Offline copies cannot detect competing
children created from another copy of the same terminal source.

```powershell
$export = '.\target\release\export_native_training_handoff.exe'
# Replace all arguments with the actual local paths and admitted unused IDs.
& $export --inspect-current $terminal
& $export --check-new-life $terminal $newWorldSeed $newOrganismId
& $export --begin-new-life $terminal $newWorldSeed $newOrganismId $newOutput
& $export --inspect-current "$newOutput\store\current.alife-checkpoint"
```

Preflight performs full native source admission and exact canonical compatibility
checks without a device, output creation or seed consumption. Creation explicitly
requires the native hardware GPU and checks the source again before publication.
Verify the imported policy and actor/value ages match the selected source exactly,
new ID/world seed, tick zero, zero new-life training counts, unchanged source hash,
and unchanged reserved exclusions/tail. Source provenance hashes are integrity
bindings, not authentication of a file supplied by someone else.

For a first short real-world smoke, use the existing `--resident-continuous` with
`STORE = $newOutput\store` and a normal request JSON. Its seed range starts at the
next unused pending entry (after the two initialization entries); do not reserve
across an already owned tail. Use a small real decision/update budget, explicit
collection/finalization deadlines within the resource window, and the ordinary
output quota. Use `--recover-resident` and `--finish-resident-run` as in the existing
owner protocol; the external owner alone publishes the returned candidate.
Preserve the immutable new-life baseline and terminal source through recovery.

This transition does not fix action selection or demonstrate feeding competence.
A food candidate, teacher cue, visible object, reachable object, selected command,
accepted world action and actual nourishment are separate observations. Report
only observed outcomes. Do not force a meal or replay a near-starved old body to
make this qualification appear successful.

## Cloud evidence

Source-only cloud work uses disposable metadata fixtures for typed lineage/codec
checks, and a real native CPU newborn world for collection, recovery and next-Adam
continuity. The existing CPU collector rejects the GPU live-policy compiler hash;
the component smoke explicitly binds only its disposable actor metadata to the
ordinary founder graph while preserving weights, moments, ages and masks. It is
not an end-to-end test of GPU Current admission. Production transition code never
performs that metadata rebind. Metadata placeholders are explicitly rejected by full native asset
admission. These tests do not certify the actual saved PC checkpoint, native GPU
birth/restore, feeding capability or all-night survival. Those require the owner
qualification above before training is resumed.
