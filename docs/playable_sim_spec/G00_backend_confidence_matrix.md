# G00 Backend Confidence Matrix

Status: G00 baseline matrix for the playable-sim product phase.

Confidence scale:

- High: implemented and validated as backend/headless behavior.
- Medium: implemented with fixture, smoke, or optional/manual constraints.
- Low: contract, scaffold, fake provider, or missing product surface.

| Subsystem | Current evidence | Scaffold, dummy, or manual pieces | Product requirement | Confidence | Closure plan |
|---|---|---|---|---|---|
| Core IDs, math, validation, schemas | `alife_core` engine-independent contracts and validation tests | Product UI does not consume most state yet | Stable core remains oracle for all gameplay | High backend, low product UX | G03, G05, G14 |
| Brain classes, lobes, routing | Scalable brain classes and core routing metadata | No live visual scheduler or product lobe telemetry | Game loop uses class/routing data without fixed-2048 assumptions | High backend | G03, G12, G18 |
| Genome and weight split | Genetic/lifetime separation and immutable genetic baseline policy | No player-facing breeding/lifecycle gameplay | Visible reproduction/lineage uses valid genomes only | High backend, low gameplay | G09 |
| Drives, hormones, chemistry | Bounded endocrine/drive contracts and reference tick integration | Needs readable feedback in game | Player can understand needs, fatigue, sleep, pain, and reward | High backend, low UX | G04, G05, G06 |
| Sensory contexts | Engine-neutral sensory ABI and headless gathering | No visible presentation binding yet | Rendered objects map deterministically into sensory context | Medium-high backend | G02, G03 |
| Action arbitration | Structured commands and deterministic arbitration | Product controls/debug UI not wired | All player/debug/teacher/semantic paths preserve arbitration | High backend | G03, G10, G14 |
| ExperiencePatch | Sealed three-phase runtime record | Product timeline/debug view missing | Visible loop emits sealed patches before learning/logging | High backend | G03, G14 |
| Packed logging | Export/replay/logging frames and side buffers | Offline tooling only, not product telemetry UI | Gameplay can summarize logs without changing cognition | High backend, medium tooling | G14, G20, G22 |
| Memory expectancy | Bounded memory bank, recall as bias only | No player-facing memory view | Inspector/timeline shows memory influence without action replay | High backend, low UX | G05, G14 |
| Topology and curiosity | Concept/simplex/gap metadata and curiosity bias | No visible cognition map | Debug timeline shows curiosity without bypassing arbitration | High backend, low UX | G14 |
| CPU neural oracle | Deterministic sparse CPU math and reference tick loop | Not presented as live creature behavior | Game tick bridge uses CPU as correctness oracle | High backend | G03 |
| Sleep consolidation | Deterministic sleep/offline consolidation and edit batches | Sleep is not a readable gameplay state | Sleep/wake is visible and affects survival loop | High backend, low gameplay | G06 |
| Headless world harness | Deterministic world, actions, outcomes, telemetry | Scenario fixtures are small/scripted | Player-facing world is visible and coherent | Medium-high backend | G02, G06, G07 |
| Scenario suite and golden traces | Determinism and replay tests | Not a product scenario suite | Playable scenarios cover actual game loops | Medium | G19, G22, G23 |
| Benchmark tiers | CPU smoke tiers and manual higher tiers | GPU/product performance evidence not measured here | Population targets have honest measured reports | Medium backend, low product perf | G12, G18, G23 |
| Bevy/Avian adapter | Optional crate and smoke example | No full app shell, render loop, camera, UI, or physics gameplay | Playable graphical app with feature gates | Low product | G01, G02, G04, G05, G21 |
| School/teacher | Perception-only verifier paths and smoke demo | No lesson UX or curriculum play surface | Player can run lessons without motor bypass | Medium backend, low UX | G10, G20 |
| Semantic/Gaussian | Optional fake provider and tolerated absence | No real provider UX or SLM product path | Optional meaning layer is explicit and safe | Low product | G11 |
| GPU backend | Buffer contracts, parity paths, runtime fallback | Manual hardware evidence absent; product telemetry immature | Optional GPU path with honest telemetry and no-readback rule | Medium contract, low product perf | G12, G18, G21, G23 |
| Structural recompaction | Sleep/offline double-buffer contract | Diagnostic/offline only | Product runtime preserves behavior during sleep/offline swaps | Medium | G12, G18 |
| Save/config/assets | Versioned portable saves/configs/manifests | No save slot or menu UX | Player can save/load with stable IDs and clear errors | Medium backend | G15, G16 |
| Offline logs/tools | CLI tooling and reports under `target/` | Not runtime/player-facing | Useful QA/debug support without runtime dependency | Medium tooling | G13, G19, G22 |
| ETF/NC and generated weights | Optional research/asset paths | Not required for gameplay | Optional content can be generated without heavy runtime deps | Low product | G16, G19 |
| Evolution/genome lab | Offline breeding/mutation tooling | Not active gameplay lifecycle | Valid lineage gameplay and population pressure | Medium tooling, low gameplay | G09 |
| P35 playground | Headless CLI examples and manifest | No graphical user experience | Coherent user-facing demos and docs | Medium integration, low product | G01-G06, G20 |
| Release gate | P36 checklist, fast soak, final status report | Manual GPU/graphics gates remain hardware-dependent | Product RC has complete evidence and limitations | High process, low product completeness | G21-G24 |

## First Executable Plan

`G00` is the first executable plan in `plan_manifest.json`. Its `next` field
points to `G01`, and `G01` depends on `G00`. The imported
`prompts/NEXT_PROMPT_G00.md` instructs running G00 only and stopping before G01.

## Product Confidence Freeze

G00 freezes the backend as a validated foundation, not as a playable game. G01
is the next plan because the first missing product layer is the graphical app
shell and feature-gated launcher.
