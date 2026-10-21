# Current implementation map

> This guide maps the current implementation. The [organism and gameplay design](architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md) describes the intended behavior; current layouts and constants remain replaceable.

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
- `alife_gpu_backend` owns gameplay neural state and WGSL execution.
- The approved offline CPU cloud trainer in `alife_training/cpu-backend` executes
  native graph/learning semantics in an explicit CPU evidence domain. This placement
  follows AOA-ADM-007 and AOA-TIME-006; gameplay GPU authority remains unchanged.
  Native handoffs preserve weights, moments and exact integer ages; qualification
  limits are recorded in the [CPU backend report](performance/20261019-native-cpu-training-backend.md).
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

Ordinary play uses cached memory cues for attention and one focused episodic
recall every four ticks or on significant changes, searching at most sixteen
records per channel. Concept summaries and quiet topology observations run
every ten ticks. Immediate senses, neural selection and GPU outcome learning
remain at tick rate. Each candidate receives a fresh current-frame decision key,
even when its expectancy is cached. Foundation training retains the audited
per-frame path with batched GPU mutable-state capture (`capture_training_state_batch`).
Candidate memory admits consequences, surprises, manipulation, blocked attempts,
and first novel inspection rather than routine movement ticks.

Action arbitration uses signed `ActionCandidate` and `SpeechPayload` decoder projections
and signed `TemporalPredictive -> ActionPlanning` (`MotorProposal`) tracts, with dedicated
motor neuron populations per `CandidateActionFamily` (`Ingest`, `Contact` for `Grab`,
`Other` for `Play`, `Manipulate`, `Locomote`, and `Avoid`). Innate `Approach` priors
are valence-aware via chemical/recalled valence (`feature 15`), and `Avoid` includes a
non-contact spatial retreat fallback.

GPU unavailability is typed unavailability. CPU reference helpers do not take over the production neural policy.

## Inherited biochemical learning signals

Chemical dynamics remain in the inherited sparse graph: species baselines and
decay, reactions, emitters, and receptors. The same graph carries a
`BiologicalValueProfile` for measured drive/hormone changes, energy, injury, and
failed-attempt disappointment. Its two homologs use the existing chemistry
inheritance path. A sealed biology snapshot stores the expressed profile and
validates it against its phenotype; the world cannot choose those weights.

Baseline dopamine and cortisol modulate measured consequences rather than
rewarding/punishing an action merely for existing. In WGSL waking fast plasticity
(`closed_loop_plasticity.wgsl`), Oja homeostatic normalization is gated by
`clamp(abs(local_third_factor * credited_eligibility), 0.0, 1.0)` so unrewarded
activity does not decay `genetic + lifetime` weights. The founder PPO objective
uses this same causal frame and the inherited action-credit receptor profile.

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
