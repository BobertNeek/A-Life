# Current implementation map

> **SUPERSEDED AS ARCHITECTURE:** This file is a non-normative map of the
> current implementation. The sole controlling architecture is
> `architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`.
> Current crates, GPU placement, layouts, adapters, constants, and gaps do not
> amend v2.0.

## Authority model

A-Life separates cognition, world truth, and presentation.

```text
world perception + unscored legal candidates
                    |
                    v
       GPU neural selection and learning
                    |
                    v
 world validation -> execution -> sealed outcome
                    |
                    v
      read-only presentation projection
```

- `alife_world` owns perception facts, legality, candidate construction, action execution, outcomes, ecology, speech events, persistence, and world identity.
- `alife_gpu_backend` owns production neural state and WGSL execution.
- `alife_game_app` schedules the live loop and translates authoritative state into player-facing surfaces.
- Renderers and UI never own simulation truth.

## Workspace map

| Crate | Responsibility |
| --- | --- |
| `alife_core` | Engine-neutral ABI, genomes, phenotypes, foundations, capacity, memory, language, and reference helpers |
| `alife_world` | Authoritative world, ecology, perception, candidates, actions, outcomes, speech, persistence, and voxel snapshot data |
| `alife_gpu_backend` | GPU buffers, WGSL pipelines, selection, plasticity, memory, topology, sleep, and closed-loop runtime |
| `alife_runtime` | Shared GPU session and checkpoint authority |
| `alife_archive` | Immutable birth/life manifests, content-addressed assets, lineage, and learned checkpoints |
| `alife_school` | Teacher, curriculum, nursery, readiness, and verification contracts |
| `alife_semantic` | Private local SLM prior and bounded speech translation |
| `alife_training` | Offline training, evaluation, and evolution tools |
| `alife_bevy_adapter` | Bevy-facing identity and integration helpers |
| `alife_tools` | Developer tools and durable machine-readable reports |
| `alife_game_app` | CLI, headless smokes, production Bevy shell, GPU live runtime, controls, and voxel presentation |

## Neural tick

For each admitted organism, the production live runtime:

1. reconciles stable organism IDs with GPU residents;
2. gathers the current grounded perception frame;
3. asks the world for unscored legal candidates;
4. binds memory and topology context without adding host-authored action scores;
5. dispatches WGSL neural encoding, recurrent dynamics, candidate scoring, and winner selection;
6. binds the selected index back to the exact candidate;
7. asks the world to validate and execute the command;
8. seals the outcome and applies GPU learning, memory, and topology updates;
9. advances the world after the batch.

GPU unavailability is typed unavailability. CPU reference helpers do not take over the production neural policy.

## Production voxel startup

The production frontend performs a GPU and content preflight, selects or materializes a portable save, restores a required GPU runtime, installs a GPU tick system, and spawns the voxel scene.

The current split is important:

- the GPU runtime owns a live `HeadlessWorld`;
- the renderer loads its initial visual records from the source save;
- the GPU runtime publishes immutable, tick-bound presentation frames;
- the renderer projects matching live transforms and creates missing newborn presentation roots;
- runtime retirement events remove the matching Bevy hierarchy and identity-map entry;
- reconciliation clears stale selection, follow, and scene records;
- procedural animation remains local presentation layered on the authoritative root position.

The adapter is read-only with respect to world truth. Presentation loss cannot change the organism, action, or lifecycle result.

The active graphics stack uses one layered-grid terrain path, one lighting path, batched terrain surfaces, lazily created overlay meshes, shared effect assets per effect kind, and modular creature parts. Dynamic overlay contents and VFX trigger selection remain launch-snapshot derived and are explicitly flagged in source.

## Persistence

Portable saves can carry optional GPU runtime state. Validation binds a sealed checkpoint to save identity and world tick, forbids bulk neural readback as gameplay authority, and stops on invalid state. Atomic replacement keeps voxel persistence and GPU checkpoint identity aligned.

Checkpoint modes remain distinct:

- `GeneticRebuild` reconstructs inherited cognition from foundation plus genome.
- `DurableLearnedFounder` preserves selected consolidated learning and durable memories while clearing transient and world-local state.
- `ExactResume` restores a sealed runtime boundary.

## Archive and lifecycle

The runtime supports two required transactions:

- create the immutable genetic birth archive before GPU insertion;
- seal final outcome and life statistics, optionally capture learned state, archive the life, then retire the GPU handle and remove the world entity.

These methods and presentation retirement hooks are implemented. A fresh production run still needs to prove the complete autonomous birth, archive, retirement, and reload sequence.

## Teacher and local SLM

Teacher input is ordinary spatial perception: speech, glyphs, objects, gestures, demonstrations, and visible consequences. The teacher may privately grade and plan, but creature-facing feedback becomes a world event.

The private local SLM has two bounded uses:

- weak developmental prior requests;
- surface translation between human text and already bounded token schemas.

It cannot select actions, choose targets, inject reward, change weights, create hidden concepts, or bypass world perception. The active product uses optional SLM speech translation; it does not feed a local SLM prior into live GPU cognition.

## Failure semantics

Neural dispatch failure makes that neural action unavailable. The Bevy shell currently records unavailable telemetry rather than necessarily closing the application. Documentation and UI must distinguish neural failure-stop from process termination.
