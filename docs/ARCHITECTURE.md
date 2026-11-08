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
- The opt-in offline CPU trainer in `alife_training/cpu-backend` executes native
  graph and learning semantics. Its results identify CPU execution separately
  from the gameplay GPU path.
  Native handoffs preserve weights, moments and exact integer ages; qualification
  limits are recorded in the [CPU backend report](performance/20261019-native-cpu-training-backend.md).
- `alife_game_app` schedules the live loop and translates authoritative state into player-facing surfaces.
- Renderers and UI never own simulation truth.

## Workspace map

| Crate | Responsibility |
| --- | --- |
| `alife_core` | Engine-neutral ABI, genomes, phenotypes, foundations, capacity, memory, language, and reference helpers |
| `alife_world` | Authoritative world, ecology, perception, candidates, actions, outcomes, speech, persistence, and voxel snapshot data |
| `alife_gpu_backend` | GPU buffers, WGSL neural execution, plasticity, sleep, and state capture |
| `alife_runtime` | Shared GPU session and checkpoint authority |
| `alife_archive` | Immutable birth/life manifests, content-addressed assets, lineage, and learned checkpoints |
| `alife_storage` | Optional shared logical-byte budgets for exclusively owned temporary output trees |
| `alife_school` | In-world language nursery and speech exposure |
| `alife_semantic` | Private local SLM prior and bounded speech translation |
| `alife_training` | Offline neural training, replay, and CPU/GPU state handoff |
| `alife_tools` | Developer asset tools, persistence validators, and GPU diagnostics |
| `alife_game_app` | CLI, headless smokes, production Bevy shell, GPU live runtime, controls, stable render identity, and voxel presentation |

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
per-frame path with batched GPU mutable-state capture (`capture_training_states`).
Candidate memory admits consequences, surprises, manipulation, blocked attempts,
and first novel inspection rather than routine movement ticks.

Action arbitration uses signed `ActionCandidate` and `SpeechPayload` decoder projections
and signed `TemporalPredictive -> ActionPlanning` (`MotorProposal`) tracts, with dedicated
motor neuron populations per `CandidateActionFamily` (`Ingest`, `Contact` for `Grab`,
`Other` for `Play`, `Manipulate`, `Locomote`, and `Avoid`). Innate `Approach` priors
are valence-aware via chemical/recalled valence (`feature 15`), and `Avoid` includes a
non-contact spatial retreat fallback.

GPU unavailability is typed unavailability. CPU reference helpers do not take over the production neural policy.

## Rest and realized translation

The world combines a factorized motor bundle into one body event and advances
biology once. Selecting `Rest` does not grant sleep recovery when another
part of that interval actually translates the organism. Before the
biology advance, the world checks each realized displacement and clears only
`sleep_recovery` if any segment is nonzero. Separate path segments matter:
opposite movements still count as work even when their net displacement is zero.

A blocked move with no displacement can coexist with successful stationary
rest. Its existing attempt cost remains charged. Movement costs, nutrition,
contact, damage, and other doses are not replaced or applied twice. This rule
does not change inherited chemistry, neural action selection, or saved state.
This check is translation-only. An in-place body turn changes body yaw and may
move a carried object, but currently reports zero organism displacement, so
`Turn` plus `Rest` can still receive recovery. Head/gaze orientation is distinct
from body turning. A future yaw rule must use realized body rotation and preserve
blocked-turn semantics, rather than rejecting a selected command.

Scheduled sleeping/no-decision intervals admit a separate physical contact
response for their explicit organism IDs. Fresh damaging contact suppresses
physical recovery even when withdrawal is blocked. Localized sources determine
an outward direction; fresh unlocalized damage uses bounded backwards translation
without changing body yaw. Quiet sleep and chronic pain without fresh damage do
not trigger this response. Awake residents with no selection are not included,
and a Completed consolidation held for durability remains eligible even though
it receives no recovery dose.

This body response neither wakes cognition nor creates a voluntary episode or
learning credit. The scheduler, consolidation payload and neural writes retain
their ordinary progression and capture obligations. The enclosing staged tick
owns rollback; each due body advances biology once, and bodies already advanced
by a voluntary transaction are skipped. Current body state is available through
normal later perception. The host regression fixtures use scalar driver identities
and do not prove device execution. Separate `sleeping_body_software_runtime_`
fixtures use an explicitly injected CPU Vulkan test backend to exercise runtime
code and exact capture/restore. They do not qualify ordinary hardware admission,
which continues rejecting software adapters.

The `rest_motion_` world regressions cover moving rest with and without a held
object, stationary and blocked rest, energy parity with movement alone, one
biology advance, rollback, and cancelled-path normalization. Run them with
`cargo test -p alife_world --lib rest_motion_ -- --test-threads=1`.

## Inherited biochemical learning signals

Chemical dynamics remain in the inherited sparse graph: species baselines and
decay, reactions, emitters, and receptors. The same graph carries a
`BiologicalValueProfile` for measured drive/hormone changes, energy, injury, and
failed-attempt disappointment. Its two homologs use the existing chemistry
inheritance path. A sealed biology snapshot stores the expressed profile and
validates it against its phenotype; the world cannot choose those weights.

Baseline dopamine and cortisol modulate measured consequences rather than
rewarding/punishing an action merely for existing. WGSL waking fast plasticity
(`closed_loop_plasticity.wgsl`) and the opt-in native CPU backend use the same
per-synapse credit `q = local_third_factor * credited_eligibility`, with
`delta_fast = learning_rate * alpha * q - normalization_rate * abs(q) * post^2 * effective`.
Thus zero local credit does not cancel expressed genetic/lifetime strength through
fast weights; existing fast-weight bounds still apply. This corrects future CPU
updates without rewriting acquired state, changing GPU arithmetic or migrating
checkpoints. The founder PPO objective
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

### Entity locomotion clearance

Entity obstacles block body motion with a closed 3D swept sphere centered at the
object's authored position, using authored obstacle radius plus the terrain's
`LocomotionLimits.body_radius` (default 0.2 without terrain). The same expanded
radius determines admission, initial overlap, tangency and earliest-surface
contact, with entity-ID ties stable. Local steering uses authored obstacle radii
plus the same body radius, or its existing larger conservative carrying envelope.
This is an isotropic approximation: body radius changes vertical as well as
horizontal clearance, including elevated obstacles and sloping trajectories.
Terrain-AABB footprint validation remains separate and unchanged. Legacy
hand/contact reach, tactile sensing, visual centers, hazard contact, and carried
pose/sweep checks retain their own existing geometry; this does not introduce
anatomical height or hand thickness. Save fields are unchanged, but restored
worlds continue under the corrected motion clearance.

Old saves and editor poses can already overlap an expanded entity sphere. A
nonzero body move may escape a strict overlap only when squared center distance
increases throughout the segment (initial radial dot product is nonnegative).
At an exact boundary the move must point strictly outward; stationary, inward,
and boundary-tangent moves remain blocked. Each obstacle is checked separately,
including obstacles farther along the escape segment. Bounded local steering
uses this same directional rule on each actual candidate displacement, including
the exact original approach X/Z endpoint and terrain-projected endpoint height,
without bypassing its view or detour limits,
terrain checks, or conservative carrying envelope.

Player placement uses a separate static rule for agents: new destinations may
not create expanded body overlap with an entity obstacle. A preexisting overlap
may be kept or improved, per obstacle, while air carrying stays endpoint-only.
Initial pickup, unchanged projected ground, and release to the accepted ground
remain available for recovery, including legacy held saves and editor geometry
changes during a hold. Thus a released pose is not guaranteed overlap-free.
Terrain support still uses the declared body limits, and item placement and
carried-grip validation retain their existing geometry. No save migration or
new persistent field is needed.

Steering projects candidate endpoint heights with the immutable terrain height
query before overlap escape classification. It does not run multiple terrain
move validations: only the selected candidate receives the full authoritative
support, slope, water and swept-path checks. Unsupported endpoints are not made
legal by projection, and worlds without terrain retain their actual 3D candidate.
