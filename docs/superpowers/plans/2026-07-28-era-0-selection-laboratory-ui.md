# Era 0 Selection Laboratory UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the production Lineage Library text wall with a responsive, evidence-backed Era 0 selection laboratory and HabitatAuthority controls.

**Architecture:** Keep presentation and input in `alife_game_app`. Build structured Bevy UI sections from real `LineageLibrary` rows and the read-only `HabitatPresentationProjection`; route every habitat command through `HabitatAuthority`, preserving typed `Unknown` evidence and `NeuralClosedLoopGpu` identity.

**Tech Stack:** Rust 2021, Bevy 0.18 UI, `alife_archive`, `alife_world`, Vulkan production runtime.

## Global Constraints

- Preserve Y/Escape and existing speech controls.
- Never invent archive, evaluation, habitat, relationship, speech, or provenance evidence.
- Keep founder cohorts distinct and bounded to 4-16 members before world creation.
- Do not mutate neural state, actions, rewards, semantic answers, or selection scores.
- Keep Wild breeding creature-chosen and all four habitat modes `NeuralClosedLoopGpu`.

---

### Task 1: Structured Lineage Library

**Files:**
- Modify: `crates/alife_game_app/src/production_conversation_lineage_ui.rs`

**Interfaces:**
- Consumes: `LineageLibrary::latest_manifest_digests`, `load_manifest`, and `load_life_statistics`.
- Produces: source/data filters, stable sorting, archive details, founder modes, and bounded cohort state rendered as separate Bevy UI sections.

- [x] Add unit tests for real row mapping, `Unknown`, filter/sort behavior, 4-16 cohort validation, closed visibility, and 1920x1080/1366x768 layout bounds.
- [x] Run the focused lineage test and confirm the missing structured hierarchy/view-model behavior.
- [x] Implement the minimal structured nodes, row/view mapping, input, and responsive layout needed to pass.
- [x] Re-run the focused test and commit the coherent UI slice.

### Task 2: Habitat Evidence and Controls

**Files:**
- Modify: `crates/alife_game_app/src/production_conversation_lineage_ui.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`

**Interfaces:**
- Consumes: `HeadlessWorld::habitat_presentation_projection` and `HabitatAuthority::{tag_creature,authorize_operation,authorize_breeding,transfer}`.
- Produces: active-creature habitat details, real affinity/typed trust-fear/speech evidence, and player operation receipts or explicit rejections.

- [x] Add failing tests for projection mapping, ungrounded speech remaining `Unknown`, mode-specific routing, rejection text, and untouched Wild breeding.
- [x] Implement runtime-local routing through cloned/replaced `HabitatAuthority` state and structured habitat UI controls.
- [x] Run the focused tests and commit the habitat slice.

### Task 3: Production Evidence

**Files:**
- Create: `docs/superpowers/assets/era0-selection-lab-1920x1080.png`
- Create: `docs/superpowers/assets/era0-selection-lab-1366x768.png`
- Modify: player-facing documentation only where controls or evidence paths need recording.

**Interfaces:**
- Consumes: the production `alife_game_app` Vulkan launch path and real Y-key laboratory input.
- Produces: two committed runtime screenshots plus exact GPU and test receipts.

- [x] Run the focused library and habitat tests, then one `alife_game_app` production-frontend boundary gate.
- [x] Launch the production Vulkan app serially, open the lab with Y, and capture both target resolutions.
- [x] Compare both screenshots with the committed blueprint, fix the largest visible discrepancy, and repeat only affected checks.
- [x] Commit final evidence, verify a clean branch, and report commits, tests, adapter/runtime evidence, and screenshot paths.

## Verification Evidence

- Focused UI gate: `cargo test -p alife_game_app --features bevy-app,gpu-runtime production_conversation_lineage_ui --lib -- --nocapture` — 13 passed, 0 failed.
- Production-frontend boundary gate: `cargo test -p alife_game_app --features bevy-app,gpu-runtime production_voxel_frontend::tests --lib` — 11 passed, 0 failed.
- Production build: `cargo build -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app` — exit 0.
- Real runtime receipt: `GpuAuthoritative` on `NVIDIA GeForce RTX 3050`, Vulkan, discrete GPU, NVIDIA driver `581.80`; authoritative execution true, failure stops learned actions true, finite rejections 0.
- Real Y-key open-state captures: `docs/superpowers/assets/era0-selection-lab-1920x1080.png` and `docs/superpowers/assets/era0-selection-lab-1366x768.png`. The window decorations make the PNG canvases 1936x1119 and 1382x807 while the requested render clients remain 1920x1080 and 1366x768.
- Blueprint comparison fix: the open laboratory now uses an opaque surface inset one percent from each edge. Normal HUD text does not bleed through the laboratory, and the evidence column has more room. The hidden closed state is unchanged.
- Diagnostic full-library run: 210 passed and 5 tests failed outside the lab and production-frontend acceptance surface (`paired_memory_probe_is_valid_for_every_promoted_class_and_profile`, `checked_in_n2048_assets_decode_for_every_production_sensor_profile`, and three `ca11_player_sandbox_editor_*` tests). The branch does not alter those failing assertions, evidence assets, or fixtures.
