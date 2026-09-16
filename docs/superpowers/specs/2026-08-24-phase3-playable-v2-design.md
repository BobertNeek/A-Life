# Phase 3 architecture-honest playable v2.0 design

**Status:** Approved by the Phase 3 brief and supervisor checkpoint.

**Controlling source:** `docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`

## Goal

Ship the first small, persistent, architecture-valid A-Life v2.0 world. A player
can create it, watch six canonical organisms run through the production GPU
cognition loop, inspect them, place food, and save and load exact state.

The milestone ends when this vertical slice works. It does not add training,
fitness optimization, broad ecology, teaching, or reproduction.

## Authority boundaries

- `alife_world` owns new-world contents, founder construction, world admission,
  resources, hazards, legality, physical outcomes, and organism biology.
- `GpuLiveBrainRuntime` owns production GPU residency, cognition, personal
  memory sidecars, local learning, sleep, and exact GPU checkpoints.
- `alife_game_app` coordinates launch and transactions. It does not invent
  organisms or choose creature actions.
- Bevy and the voxel renderer consume canonical state or versioned derived
  views. They do not advance creature state independently.
- Existing foundation assets and weights remain unchanged.

This preserves AOA-INV-011, AOA-AUTH-001, AOA-FOUND-002,
AOA-PORT-004, and AOA-GAME-007.

## Canonical New Game transaction

Add a versioned `CanonicalNewGameConfig` and bootstrap operation in
`alife_world`. The configuration contains a deterministic world seed, founder
count, brain class, sensor profile identity, and world recipe version. Founder
count accepts 4 through 8 and defaults to 6. Phase 3 uses Nano512 and the
existing promoted Nano512 foundation.

For each founder, the world bootstrap performs one ordered construction:

1. Derive a unique deterministic conception seed and stable organism identity.
2. Bind the existing foundation identity to `CreatureGenome`.
3. Express the genome into `CreaturePhenotype`.
4. Construct typed organs, biochemical graph state, developmental state, and
   embodiment through `WorldOrganismRecord::newborn`.
5. Spawn the corresponding agent object with a stable world identity.
6. Register the organism record and admit it to the initial Wild habitat.
7. Validate the complete organism subsystem state graph and world bindings.

The same bootstrap creates a compact meadow with food, renewable resource
tracking, obstacles, and grounded hazards. These are ordinary world objects.
Food transfers material and energy through the existing action transaction.
Hazards produce physical contact, injury, pain, threat chemistry, and measured
physiology through that same transaction.

The world bootstrap returns the canonical world plus a receipt containing the
requested count, admitted identities, genome and phenotype identities, and
world-state digest. It never creates GPU handles or renderer entities.

The app lifecycle coordinator then completes New Game as one fail-closed
operation:

1. Convert the canonical world into a versioned save candidate using the
   selected runtime configuration and validated asset manifest.
2. Start the real production GPU backend and reconcile every admitted founder.
3. Compile or bind the existing foundation phenotype, allocate one GPU brain,
   and create one personal memory and topology sidecar per organism.
4. Attach lineage archive ownership before the first gameplay tick.
5. Write an exact checkpoint for every resident and publish the durable save
   atomically.
6. Reopen and validate the published save before the graphical world starts.

No player-visible save is published if any founder, GPU brain, memory sidecar,
archive record, or checkpoint fails. An existing target save is not silently
overwritten. This satisfies AOA-PERSIST-001, AOA-PERSIST-004,
AOA-ART-005, and AOA-FAIL-002.

## Launch and world-source behavior

The production command gains an explicit New Game mode:

```powershell
cargo run -p alife_game_app --features production-voxel-frontend -- \
  production-voxel --new-game --population 6 --seed 240824 \
  --graphics-backend vulkan --require-gpu
```

New Game creates a deterministic durable save path and launches that world in
the same process. Normal launch continues to load an exact existing save.
Population arguments must equal the canonical population loaded or created.
The frontend never clones, truncates, resizes, or reconstructs organisms.

## Production organism loop

Phase 3 reuses the repaired `GpuLiveBrainRuntime` tick as the only autonomous
decision path:

```text
world/body/biochemistry sensing
-> attention and cognitive context
-> production GPU selection
-> factorized motor bundle
-> deterministic embodiment control
-> registered world action transaction
-> body, organ, and biochemical consequences
-> measured physiology and prediction residual
-> local learning, personal memory, and topology observation
-> fatigue, sleep, consolidation, and waking
```

No new host-authored food seeking, hazard avoidance, utility score, scripted
survival policy, action oracle, reward signal, or CPU neural fallback is added.
Deterministic locomotor stabilization, collision response, withdrawal reflexes,
and actuator safety remain body or embodiment functions.

## Player presentation and interaction

Keep the existing voxel terrain, GeneForge creature assemblies, camera,
selection, follow, pause, step, speed, save, and load controls. Phase 3 changes
their data source only where needed to expose current canonical state.

The selected-creature inspector reads the live world presentation row and the
runtime cognitive projection. It shows:

- stable organism and world identity;
- genome, lineage, foundation, phenotype, and visible breed identity;
- age and developmental stage;
- body health, typed organ condition, energy, and injury;
- important drives and biochemical signals;
- current attention target and GPU-selected motor behavior;
- sleep phase and consolidation state;
- personal memory, learning, concept, and topology counts;
- organism subsystem revisions and consistency status.

Unavailable evidence is displayed as unavailable, never reconstructed.

The `E` key places one food resource at the selected terrain location. Bevy
translates the input into a grounded position. `GpuLiveBrainRuntime` submits a
player world-edit request. `alife_world` validates capacity, position, resource
properties, and stable identity before committing the new object and resource
lifecycle record. The renderer then observes the changed canonical world.

## Persistence continuity

Manual save uses the existing sealed GPU checkpoint boundary and durable
manifest. Load stages a complete candidate runtime, validates all world,
organism, phenotype, body, chemistry, embodiment, memory, GPU, sleep, and
identity bindings, and swaps it into the live app only after every check passes.

The exact roundtrip test records organism IDs, genome and phenotype digests,
organ state, biochemical state, embodiment revision, GPU checkpoint identity,
memory digest, sleep state, lifecycle state, and world object digest before
save. It requires equality after load and one matching subsequent causal tick.

## Verification

Use RED-first focused tests and serial Cargo commands:

- canonical New Game creates exactly the requested 4, 6, and 8 founders;
- every founder follows the ordinary genome-to-world-admission path;
- a late bootstrap failure publishes nothing;
- one food action changes material, organ, biochemical, and homeostatic state;
- one hazard action changes injury, pain, threat, and measured physiology;
- production GPU ticks select and execute autonomous movement or interaction;
- sleep enters, consolidates atomically, and wakes in a bounded headless smoke;
- inspector rows come from canonical and runtime projections;
- player resource placement changes world authority, not renderer state;
- an exact save/load roundtrip preserves all named ownership and later control;
- one bounded Vulkan graphical smoke proves launch, selection, controls,
  resource placement, autonomous activity, and exact save/load.

Final verification uses targeted `cargo check`, focused tests, one short
headless smoke, one bounded graphical smoke, and screenshots. It does not run
training, corpora, evolution, long GPU journeys, or scale tests.

## Stop boundary

Teacher interaction and reproduction remain deferred unless the required loop
is already coherent and verified. Phase 3 stops at the first architecture-valid
playable milestone. No brain training or deliberate neural optimization occurs.
