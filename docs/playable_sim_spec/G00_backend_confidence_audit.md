# G00 Backend Confidence Audit

Status: G00 playable-sim product baseline audit.

Canonical spec pack: A-Life Playable Sim Game Completion Plan Pack
`2.0-expanded`.

Baseline: P01-P36 are complete on main at or after the post-P36 consistency
fix. This audit does not reopen P01-P36 and does not create P37. It separates
the validated backend/headless scaffold from the missing player-facing
graphical simulation game.

## Spec Import Verification

- Imported path: `docs/playable_sim_spec/`.
- Required files present: `README.md`, `plan_manifest.json`,
  `prompts/NEXT_PROMPT_G00.md`, and `prompts/GOAL_MODE_DRIVER_PROMPT.md`.
- Plan manifest version: `2.0-expanded`.
- Manifest plan count: 25.
- First executable plan: `G00`.
- Final listed plan: `G24`.
- G-plan files are present under `docs/playable_sim_spec/plans/` as
  `G00` through `G24`, matching the manifest `file` fields.
- `NEXT_PROMPT_G00.md` explicitly instructs creating
  `codex/G00-product-audit`, implementing G00 only, and stopping before G01.

## Current Product Readiness Summary

The repository is ready as a backend/headless/reference scaffold. It is not yet
a playable graphical sim game. The headless CPU path can run deterministic
small scenarios, produce sealed patches, validate saves/configs/assets, run
benchmark smokes, and exercise optional school, semantic, GPU fallback, and
tooling paths. The missing layer is the product game spine: a visible app,
player-facing world binding, live creature presentation, UI, survival/ecology
loop, population/lifecycle gameplay, game UX, packaging, and product QA.

## Subsystem Classification

### Core Cognitive Contracts

Classification: real implementation.

Evidence: `alife_core` contains engine-independent IDs, math, validation,
brain classes, lobe routing, genome/weight split, chemistry, sensory ABI,
action arbitration, three-phase sealed experience, packed logs, memory
expectancy, topology, CPU neural projection, reference brain tick, and sleep
consolidation contracts.

Product confidence: high for backend contracts, low for player-facing
readability because no live visual app consumes the full state yet.

Closure plan: G03 bridges the live brain loop into the game scheduler; G05 and
G14 expose state through inspectors and cognitive timelines.

### Headless World, Scenarios, and Determinism

Classification: real headless implementation with fixture-driven coverage.

Evidence: `alife_world` supplies deterministic headless world fixtures,
scenario/golden trace tests, save/load round-trip fixtures, and fast soak
coverage. P35 playground commands can run the headless scenario path.

Product confidence: medium-high for deterministic backend behavior, low for
visible game play because world state is not yet presented through a graphical
app.

Closure plan: G01-G03 build the app shell, visible world binding, and live tick
bridge. G06-G09 turn fixtures into playable survival, ecology, population, and
lifecycle loops.

### Bevy/Avian Adapter

Classification: optional adapter/smoke surface.

Evidence: `alife_bevy_adapter` is a separate optional crate. It depends on
Bevy, keeps those types out of `alife_core`, and has a smoke example.

Product confidence: low. The adapter boundary exists, but it is not a polished
playable app, renderer, input layer, camera system, or creature visualization.

Closure plan: G01 creates the graphical app shell and launcher; G02 binds
stable world entities to visible presentation; G04-G05 add creature rendering,
camera, selection, and inspection; G21 handles platform smoke and asset bundle
discipline.

### GPU Backend

Classification: optional diagnostic/parity backend with CPU fallback.

Evidence: `alife_gpu_backend` contains P24-P29 buffer contracts, static
forward parity, plasticity/Oja, routing masks, recompaction staging, runtime
selection, diagnostics, and CPU fallback policy.

Product confidence: medium for schemas and fallback behavior; low for product
GPU performance because manual hardware evidence is not recorded here and CPU
fallback is not a GPU timing claim.

Closure plan: G12 hardens product GPU runtime telemetry and no-readback
diagnostics; G18 validates population performance/LOD; G21 and G23 cover
platform and release-candidate gates.

### School and Teacher

Classification: perception-only verifier/backend contract.

Evidence: `alife_school` exposes teacher/verifier style paths and P35 includes
a school demo that verifies sealed patches without direct motor bypass.

Product confidence: medium for boundary safety, low for playable lesson UX.

Closure plan: G10 builds playable school/teacher mode and lesson UX; G20
documents onboarding and player help; G23 includes release-candidate coverage.

### Semantic/Gaussian Adapter

Classification: optional fake-provider/context boundary.

Evidence: `alife_semantic` remains optional, and P35 documents feature-gated
fake provider use. Missing providers are tolerated.

Product confidence: low for product meaning layer because no real provider UX,
SLM model management, or player-facing semantics flow exists yet.

Closure plan: G11 hardens the semantic/SLM provider boundary and optional
meaning layer; G20 documents player-facing behavior.

### Save, Config, and Assets

Classification: real portable schema contracts and tiny fixtures.

Evidence: P34 save/config/manifest fixtures are versioned, use stable IDs, and
reject incompatible schemas or missing required assets.

Product confidence: medium for schema integrity, low for player-facing save UX.

Closure plan: G15 adds slots, autosave, config menus, and save/load UX; G16
extends content authoring pipeline and asset flow.

### Offline Tools and Research Paths

Classification: optional tooling, not runtime dependency.

Evidence: `alife_tools` contains offline logs, benchmark smokes, ETF/NC,
generated weight assets, genome lab, playground CLI, and release-gate helpers.

Product confidence: medium for developer tooling; not a gameplay surface.

Closure plan: G13, G16, G19, G20, and G22 use tooling to support sandbox,
content, balance, documentation, and QA without making tools required by
runtime.

### P35 Playground

Classification: headless-first CLI examples and docs.

Evidence: `docs/playground_examples.md`, `examples/p35/playground_manifest.json`,
and `p35_playground` demonstrate headless scenario, save/load, school,
semantic optional path, GPU fallback, and manifest validation.

Product confidence: medium as an integration smoke surface; low as a player
experience. It is a CLI playground, not the game.

Closure plan: G01-G06 convert the integration spine into an actual playable
visible loop. G20-G24 close onboarding, packaging, QA, release-candidate, and
roadmap lock.

## Missing Product Gameplay

The following are not complete in the current backend scaffold:

- Graphical game shell and feature-gated launcher.
- Stable visible presentation for world entities.
- Live app tick bridge that turns backend stepping into a player-facing loop.
- Creature rendering, animation, expression, and state visualization.
- Camera controls, selection, and creature inspector.
- Playable survival loop with readable food, hazard, sleep, reward, and
  failure feedback.
- Resource ecology, terrain zones, hazards, and balancing.
- Multi-creature population and social interaction gameplay.
- Lifecycle, reproduction, death, lineage, and player-facing consequences.
- Lesson UX, semantic provider UX, cognition visualization, save/load UX,
  world editing, content pipeline, audio/VFX polish, onboarding, packaging,
  bug-bash, and release-candidate discipline.

These are assigned to G01-G24. No gameplay implementation is performed in G00.

## Integration Risks

- The Bevy app may expose hidden dependency leaks if app shell code is not kept
  out of `alife_core`.
- Product UI can accidentally bypass action arbitration unless teacher,
  semantic, debug, and inspector paths remain perception/diagnostic only.
- GPU runtime can overclaim maturity if CPU fallback reports are mistaken for
  measured GPU performance.
- Save/load UX can accidentally persist engine-local IDs unless P34 stable-ID
  rules remain mandatory.
- Graphical examples may become hardware-required unless G01 and later plans
  keep headless validation as the default path.

## G00 Verdict

PASS_WITH_NOTES for backend confidence freeze. The backend/headless foundation
is strong enough to start the playable-sim product phase, but product gameplay
confidence remains intentionally low until G01-G06 establish the visible app
spine and first playable survival loop.
