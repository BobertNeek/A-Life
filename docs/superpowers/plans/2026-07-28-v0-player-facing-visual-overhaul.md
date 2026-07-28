# V0 Player-Facing Visual Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the real production voxel app into a coherent player view with warm art direction, readable creatures, restrained HUD, honest creature-state cues, and verified screenshot evidence.

**Architecture:** Keep `alife_world` and the GPU brain authoritative. Add one focused presentation model in `alife_game_app`, project real saved/runtime state into it, and render a compact Bevy HUD beside the existing world scene. Reuse the existing selection, camera, follow, save, and GPU tick paths; add only view recovery and screenshot naming needed for player-facing acceptance.

**Tech Stack:** Rust 2021, Bevy 0.18 UI/PBR, existing A-Life production voxel renderer, Image 2 blueprint, Cargo tests, real Vulkan screenshots.

## Global Constraints

- Authority order is `docs/master_spec.md`, `docs/architecture_decisions.md`, the approved 2026-07-28 selection design, then the active evolutionary-intelligence roadmap.
- Production cognition remains GPU-authoritative WGSL. Do not add a CPU neural shadow, parity gate, or automatic neural fallback.
- Touch only `crates/alife_game_app`, its production visual assets, minimal presentation code in `crates/alife_bevy_adapter` if required, and the directly applicable V0 docs.
- Do not change `alife_core`, `alife_gpu_backend`, `alife_tools`, or neural/world authority.
- Do not fabricate speech tokens, pairwise bonds, memories, traits, or activity. Render only state already exposed by the production save or live GPU summaries.
- Preserve `MinimumSettings30x30`, `MinSpecComfort1080p`, real save/load, stable IDs, and renderer display-only boundaries.
- Actual production screenshots at 1920x1080 and 1366x768 are visual acceptance. A mockup, compile, or headless table is not acceptance.

---

## File Structure

- Create `crates/alife_game_app/src/v0_player_experience.rs`: pure presentation vocabulary, responsive layout, readable need bands, action labels, and player-panel text derived from real values.
- Modify `crates/alife_game_app/src/lib.rs`: export the V0 presentation contracts.
- Modify `crates/alife_game_app/src/bevy_shell.rs`: retain the most recent real per-creature GPU action summary for the renderer; do not change GPU decisions.
- Modify `crates/alife_game_app/src/production_voxel_renderer.rs`: render the clean HUD, inspector, intent bubble, selection ring, view recovery, and resolution-specific screenshot paths.
- Modify `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`: integration checks for visible product UI, display-only state, responsive layout, readable selection, and hidden developer chrome.
- Modify `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_AUDIT.md`: record the new baseline failure and final screenshot comparison.
- Modify `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_COMPLETION.md`: replace stale completion claims with the V0 branch receipt and exact evidence.
- Use `docs/superpowers/specs/assets/v0-player-experience-blueprint.png`: approved Image 2 blueprint for hierarchy, warmth, creature focus, and HUD density.

### Task 1: Honest Player Presentation Model

**Files:**
- Create: `crates/alife_game_app/src/v0_player_experience.rs`
- Modify: `crates/alife_game_app/src/lib.rs`

**Interfaces:**
- Consumes: real creature label, `BrainScaleTier`, `ActionKind`, homeostatic drive values, social affinity, memory/concept/gap counts, learning enablement, and last consolidation tick.
- Produces: `V0PlayerHudLayout`, `V0PlayerCreatureInput`, `V0PlayerCreaturePresentation`, `v0_player_hud_layout((u32, u32))`, and `v0_player_creature_presentation(&V0PlayerCreatureInput)`.

- [ ] **Step 1: Write failing unit tests for layout and honest cues**

```rust
#[test]
fn representative_resolutions_keep_the_world_visible() {
    let wide = v0_player_hud_layout((1920, 1080));
    let compact = v0_player_hud_layout((1366, 768));
    assert!(wide.inspector_width_px <= 360.0);
    assert!(compact.inspector_width_px <= 300.0);
    assert!(compact.compact);
}

#[test]
fn presentation_uses_only_supplied_creature_state() {
    let view = v0_player_creature_presentation(&fixture_input(ActionKind::Vocalize));
    assert_eq!(view.voice, "Vocalizing");
    assert!(view.inspector_text.contains("Memory 3"));
    assert!(!view.inspector_text.contains("friend"));
    assert!(!view.inspector_text.contains("GPU"));
}
```

- [ ] **Step 2: Run the focused unit tests and confirm the missing-module failure**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app v0_player_experience --lib -j 1 -- --nocapture`

Expected: FAIL because `v0_player_experience` and its contracts do not exist.

- [ ] **Step 3: Implement the pure presentation model**

```rust
pub fn v0_player_hud_layout(resolution: (u32, u32)) -> V0PlayerHudLayout {
    let compact = resolution.0 < 1600 || resolution.1 < 900;
    V0PlayerHudLayout {
        inspector_width_px: if compact { 286.0 } else { 340.0 },
        edge_margin_px: if compact { 10.0 } else { 18.0 },
        font_size_px: if compact { 12.0 } else { 14.0 },
        compact,
    }
}
```

Implement action labels, need urgency, display-name cleanup, social-disposition wording, and learning/memory wording from input fields only. `Vocalize` may display `Vocalizing`; it must not invent token content.

- [ ] **Step 4: Run the focused unit tests and confirm they pass**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app v0_player_experience --lib -j 1 -- --nocapture`

Expected: PASS with all V0 presentation-model tests green.

- [ ] **Step 5: Commit the presentation model**

```powershell
git add crates/alife_game_app/src/v0_player_experience.rs crates/alife_game_app/src/lib.rs
git commit -m "feat: add honest V0 player presentation model"
```

### Task 2: Preserve Real Per-Creature GPU Action State

**Files:**
- Modify: `crates/alife_game_app/src/bevy_shell.rs`
- Modify: `crates/alife_game_app/src/v0_player_experience.rs`

**Interfaces:**
- Consumes: `Vec<LiveBrainTickSummary>` returned by `GpuLiveBrainRuntime::tick()`.
- Produces: `ProductionGpuBrainAuthorityResource::recent_actions`, keyed by raw `OrganismId`, with `selected_action_kind`, success, tick, memory updates, and learning updates copied from real summaries.

- [ ] **Step 1: Write a failing test for summary-to-presentation mapping**

```rust
#[test]
fn recent_action_projection_keeps_real_action_and_outcome() {
    let event = V0RecentCreatureAction::from_live_summary(&live_summary(
        ActionKind::Inspect,
        true,
        41,
    ));
    assert_eq!(event.action_kind, Some(ActionKind::Inspect));
    assert_eq!(event.success, Some(true));
    assert_eq!(event.tick.raw(), 41);
}
```

- [ ] **Step 2: Run the focused test and confirm the missing-contract failure**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app recent_action_projection --lib -j 1 -- --nocapture`

Expected: FAIL because the real-summary projection is absent.

- [ ] **Step 3: Store the tick summaries after each successful GPU tick**

```rust
match runtime.runtime.tick() {
    Ok(summaries) => {
        authority.telemetry = runtime.runtime.authority_telemetry();
        authority.recent_actions = summaries
            .iter()
            .map(|summary| (summary.organism_id.raw(), V0RecentCreatureAction::from_live_summary(summary)))
            .collect();
    }
    Err(error) => { /* retain the existing typed unavailable handling */ }
}
```

Do not change `GpuLiveBrainRuntime`, candidate scoring, selected commands, world execution, or neural telemetry semantics.

- [ ] **Step 4: Run the focused test and GPU authority policy test**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app recent_action_projection --lib -j 1 -- --nocapture`

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_policy -j 1 -- --nocapture`

Expected: PASS; GPU-required policy remains authoritative with no fallback.

- [ ] **Step 5: Commit the runtime presentation bridge**

```powershell
git add crates/alife_game_app/src/bevy_shell.rs crates/alife_game_app/src/v0_player_experience.rs
git commit -m "feat: expose real GPU creature actions to V0 UI"
```

### Task 3: Clean HUD, Selection, Camera, and View Recovery

**Files:**
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs`
- Modify: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

**Interfaces:**
- Consumes: `V0PlayerCreaturePresentation`, `Fvr03ProductionVoxelSelectionResource`, `Fvr04ProductionCreatureSceneResource`, `Fvr04ProductionCreatureFollowResource`, and real recent GPU actions.
- Produces: always-visible product components `V0PlayerWorldChip`, `V0PlayerCreaturePanel`, `V0PlayerBottomBar`, and `V0PlayerIntentBubble`; `R` restores isometric camera, selected creature, running state, and disabled follow.

- [ ] **Step 1: Write failing integration tests for product hierarchy and recovery**

```rust
#[test]
fn v0_default_view_shows_player_hud_and_hides_developer_chrome() {
    let mut app = product_app((1920, 1080));
    app.update();
    assert_eq!(visible_count::<V0PlayerCreaturePanel>(&mut app), 1);
    assert_eq!(visible_count::<Fvr05ProductionLeftControlPanel>(&mut app), 0);
}

#[test]
fn v0_recover_restores_isometric_camera_and_valid_selection() {
    let mut app = product_app((1366, 768));
    press_key(&mut app, KeyCode::KeyR);
    app.update();
    assert_eq!(camera_mode(&mut app), Fvr03ProductionVoxelCameraMode::OrthographicIsometric);
    assert!(app.world().resource::<Fvr03ProductionVoxelSelectionResource>().selected.is_some());
}
```

- [ ] **Step 2: Run the focused integration tests and confirm the missing-component failure**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer v0_ -j 1 -- --nocapture`

Expected: FAIL because V0 product components and recovery do not exist.

- [ ] **Step 3: Render the restrained product UI and selection ring**

Create one dark-olive translucent right panel, one compact bottom strip, one small world/profile chip, and one contextual world-space intent label. Use warm cream text, moss status accents, and an amber torus selection ring. Keep all FVR05 developer panels hidden until their existing debug controls are invoked.

- [ ] **Step 4: Add view recovery without simulation mutation**

On `R`, restore the production isometric transform and projection, select the lowest stable visible creature, disable follow, clear developer panels/overlays, resume the view, and report `View recovered`. Do not reload or rewrite simulation state.

- [ ] **Step 5: Run focused V0/FVR10/FVR11 renderer tests**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer v0_ -j 1 -- --nocapture`

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer fvr10_ -j 1 -- --nocapture`

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer fvr11_ -j 1 -- --nocapture`

Expected: PASS; clean player UI is visible, developer UI stays hidden, selection/camera/recovery works, terrain and creatures remain display-only.

- [ ] **Step 6: Commit the real player view**

```powershell
git add crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs
git commit -m "feat: ship the V0 production player view"
```

### Task 4: Representative Screenshots and Final Receipt

**Files:**
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs`
- Modify: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`
- Modify: `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_AUDIT.md`
- Modify: `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_COMPLETION.md`

**Interfaces:**
- Consumes: the real production launch and screenshot capture path.
- Produces: non-colliding screenshot files for 1920x1080 and 1366x768, plus an exact branch receipt.

- [ ] **Step 1: Write a failing screenshot-path test**

```rust
#[test]
fn v0_screenshot_paths_include_non_default_resolution() {
    assert!(v0_screenshot_filename("MinSpecComfort1080p", (1366, 768))
        .ends_with("_1366x768.png"));
}
```

- [ ] **Step 2: Run the path test and confirm failure**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app v0_screenshot_paths --lib -j 1 -- --nocapture`

Expected: FAIL because resolution-specific naming is absent.

- [ ] **Step 3: Implement stable resolution-specific capture names**

Keep the existing 1920x1080 name for compatibility. Append `_1366x768` for the representative compact capture.

- [ ] **Step 4: Build the shipping executable once with isolated outputs**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo build -p alife_game_app --release --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1`

Expected: exit 0 with one release executable.

- [ ] **Step 5: Capture and inspect both real player views serially**

Run: `C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --brain-policy gpu-required --graphics-backend vulkan --require-gpu --record-performance`

Run: `C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target\release\alife_game_app.exe production-voxel --resolution 1366x768 --profile MinimumSettings30x30 --population 30 --brain-policy gpu-required --graphics-backend vulkan --require-gpu --record-performance`

Inspect the PNGs at original resolution. Compare world warmth, creature readability, selection focus, panel density, clipping, and foreground stability against `docs/superpowers/specs/assets/v0-player-experience-blueprint.png`. Fix the single largest visible discrepancy, then recapture only the affected resolution.

- [ ] **Step 6: Run the final scoped gate once**

Run: `cargo fmt --all -- --check`

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo check -p alife_game_app --all-targets --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" -j 1`

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-v0-target'; cargo test -p alife_game_app --all-targets --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" -j 1`

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`

Expected: all commands exit 0. Screenshot review confirms the actual player view, not developer UI.

- [ ] **Step 7: Update the audit and completion receipt, then commit**

Record exact screenshot paths, dimensions, hardware/adapter, observed FPS, selection/camera/recovery interactions, honest data limits, commands, and boundary results.

```powershell
git add crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_AUDIT.md docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_COMPLETION.md
git commit -m "docs: record V0 rendered acceptance"
```

## Plan Self-Review

- Spec coverage: Task 1 covers responsive hierarchy and honest needs, social, learning, memory, and voice cues. Task 2 carries real GPU-selected actions. Task 3 covers clean default HUD, selection, camera, follow, and recovery. Task 4 covers real screenshots, performance, docs, and boundary evidence.
- Placeholder scan: no deferred implementation, scaffold milestone, or mock acceptance remains. Raw speech tokens and pairwise bonds are explicitly excluded because current production presentation state does not expose them.
- Type consistency: `V0RecentCreatureAction` is produced in Task 2 and consumed in Task 3. `V0PlayerHudLayout` and `V0PlayerCreaturePresentation` are created in Task 1 and consumed by the renderer in Task 3. Screenshot naming is isolated to Task 4.
- Scope check: all production edits remain inside `alife_game_app`. The plan does not require `alife_core`, `alife_gpu_backend`, `alife_tools`, or world-authority changes.
- Acceptance check: final proof requires original-resolution production PNGs at both requested resolutions plus the focused test and boundary commands.
