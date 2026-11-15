# CPU retained-parent corner tournament (research example)

This source-only example exposes a reproducible **operator policy-calibration** workflow: native context export, one fixed Torch semantic proposal set, native calibration/cold qualification, and an at-most-four-process tournament driver. It is not acquired learning, inherited ability, a replication of a one-shot experiment, or a gameplay/default change.

No weights, genomes, foundation assets, checkpoints, Adam state, activations, Jacobians, proposals or private traces are shipped. All such inputs and generated outputs stay local. Do not commit generated outputs. The examples deliberately reject incompatible parents instead of silently converting acquired state.

## Scope and inputs

The current recipe is specific: one living n2048 CP18/capability3 resident, mode14, grip architecture5, normal age death, acquired tick/sampling counter626, healthy body, existing g0 policy and the Inspect-valence +0.4 common-base recipe. Native policy changes preserve the genotype, private recurrent/fast/lifetime state, world, RNG, Adam moments and integer ages; only permitted readout weights and authenticated founder digest change. It is portable source, **not** a generic importer for arbitrary checkpoints or architectures.

Build from the repository root:

```sh
cargo build -p alife_training --features cpu-backend --example corner_proposals --example corner_tournament
cargo test -p alife_training --features cpu-backend --example corner_tournament
```

The three data-dependent tests are ignored by default. To run them, provide your own exact-compatible inputs and a create-only request:

```sh
ALIFE_CORNER_TEST_REQUEST=/path/to/request.json ALIFE_CORNER_PROPOSALS=/path/to/proposals.json \
  cargo test -p alife_training --features cpu-backend --example corner_tournament -- --ignored --test-threads=1
```

An export request contains `asset`, `genome`, `actor_checkpoint`, `founder_checkpoint`, and a nonexistent `outdir`, all external paths. The proposal exporter expects a decoded native checkpoint JSON file; the tournament caller expects the native checkpoint directory. The exporter makes one disposable native COPY step to obtain a source-sensitive context; the original acquired parent remains untouched. It exports a calibrated common-base checkpoint and founder locally.

```sh
target/debug/examples/corner_proposals export /path/to/export-request.json
python scripts/corner_tournament/generate.py /path/to/export/NATIVE-GRAPH-CONTEXT.json /path/to/proposals.json
target/debug/examples/corner_proposals qualify /path/to/export-request.json /path/to/proposals.json
python scripts/corner_tournament/run.py target/debug/examples/corner_tournament /path/to/base-request.json /path/to/proposals.json /path/to/new-run --workers 4
```

The base tournament request contains the same asset/genome paths, exported common-base checkpoint directory and founder file, plus `outdir`, `edits: []`, `fixture_id: 0`, `horizon: 1024`, `temperature: 1.0`, and `observe: true`. The driver supplies each program's edits and both mirrored fixture IDs. All output directories are create-only. A process-success marker is not an independent biological audit: verify native cold boundaries, original-input hashes and all outcome receipts before ranking. Resource requirements depend on full trace retention; lower `--workers` rather than truncate evidence.

## Proposal semantics and an important correction

Torch **2.15.x**, CPU float32, first checks ordered native logit parity. Autograd proposes one unit Approach-minus-Avoid contrast direction for each of four semantic input lanes, plus its negative and an unchanged baseline: nine programs, no amplitude grid or outcome-conditioned proposal search. These local Jacobians are not trajectory predictions. Native calibration validates actual coordinates and preserves all other state before any real run.

GroundedObjectSlots lanes are bearing-sine0, bearing-cosine1, distance2, **chemical[1]16**. Contact is **lane18**, not16. Bearing-sine is left/planar and bearing-cosine is forward/planar relative to the current sensing/gaze observer pose, not a fixed compass or body-yaw frame. Fixture geometry separately uses saved body yaw. The original local generation mislabeled its lane16 programs `contact-plus/minus`, borrowing a constant from the older nongrounded feature layout. Those exact programs and actual outcomes remain preserved; their correct interpretation is chemical[1]-direction perturbations. This portable generator uses `chemical_1-plus/minus` labels without changing the numerical lane. Do not relabel the original runs as contact experiments or silently substitute lane18.

## Audited original generation and scene confound

The original nine programs each ran two mirrored 1,024-interval fixtures: 18,432 native intervals and 234 audited cold boundaries. All18 stayed alive; goal meals0, legal detour traversals0, injury events0. Every arm's mean utility was0.25; baseline won only the predeclared minimum-edit-norm tie break. Total actual weighted cognitive work was1,146,638,397. This is one acquired-parent/two-fixture cluster, not18 independent population samples.

A subsequent zero-step native proof exposed a scene flaw. An inherited unconsumed food remained near the creature, but a newly added wall blocked its native ingestion reachability. The food endpoint was approximately 0.551294 from the new wall center, inside the wall's 0.75 contact-solid envelope: max(wall radius 0.35, HEADLESS_CONTACT_RADIUS 0.75). This is not a combined food/wall radius. The inherited food radius was 0.30 and nutrition was 0.15; the body-motion radius of 0.20 is separate. Native food availability was true at the parent and false at both original fixture starts. A route to the newly spawned goal was geometrically clear, but this did not protect the inherited resource.

Therefore these results demonstrate infrastructure execution, **not neural maze improvement or a causal failure of neural navigation**. The source reproduces the original confounded fixture intentionally. Before using a revised scene as a benchmark, freeze and audit public scene changes against every inherited unconsumed resource; do not reset the creature or erase the old generation. A separately frozen corrected-scene comparison is not included or claimed here.

## Qualification limits

The original caller's three CPU/native tests passed, and the actual18 runs were independently audited. The portable extraction removes private includes and changes test input plumbing, generator labels, and CLI safety; its validation is reported separately. Direct compilation against pinned CPU libraries is only partial CPU qualification. It does not establish a full-workspace Cargo, GPU/gameplay, or hosted-CI pass. No default or selected-brain promotion follows from this report.
