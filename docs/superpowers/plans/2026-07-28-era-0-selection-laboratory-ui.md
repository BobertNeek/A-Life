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

- [ ] Add unit tests for real row mapping, `Unknown`, filter/sort behavior, 4-16 cohort validation, closed visibility, and 1920x1080/1366x768 layout bounds.
- [ ] Run `cargo test -p alife_game_app --features production-voxel-frontend production_conversation_lineage_ui --lib -- --nocapture` and confirm the new tests fail for missing behavior.
- [ ] Implement the minimal structured nodes, row/view mapping, input, and responsive layout needed to pass.
- [ ] Re-run the focused test and commit the coherent UI slice.

### Task 2: Habitat Evidence and Controls

**Files:**
- Modify: `crates/alife_game_app/src/production_conversation_lineage_ui.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`

**Interfaces:**
- Consumes: `HeadlessWorld::habitat_presentation_projection` and `HabitatAuthority::{tag_creature,authorize_operation,authorize_breeding,transfer}`.
- Produces: active-creature habitat details, real affinity/typed trust-fear/speech evidence, and player operation receipts or explicit rejections.

- [ ] Add failing tests for projection mapping, ungrounded speech remaining `Unknown`, mode-specific routing, rejection text, and untouched Wild breeding.
- [ ] Implement runtime-local routing through cloned/replaced `HabitatAuthority` state and structured habitat UI controls.
- [ ] Run the focused tests and commit the habitat slice.

### Task 3: Production Evidence

**Files:**
- Create: `docs/superpowers/assets/era0-selection-lab-1920x1080.png`
- Create: `docs/superpowers/assets/era0-selection-lab-1366x768.png`
- Modify: player-facing documentation only where controls or evidence paths need recording.

**Interfaces:**
- Consumes: the production `alife_game_app` Vulkan launch path and real Y-key laboratory input.
- Produces: two committed runtime screenshots plus exact GPU and test receipts.

- [ ] Run the focused library and habitat tests, then one `alife_game_app` production-frontend boundary gate.
- [ ] Launch the production Vulkan app serially, open the lab with Y, and capture both target resolutions.
- [ ] Compare both screenshots with the committed blueprint, fix the largest visible discrepancy, and repeat only affected checks.
- [ ] Commit final evidence, verify a clean branch, and report commits, tests, adapter/runtime evidence, and screenshot paths.
