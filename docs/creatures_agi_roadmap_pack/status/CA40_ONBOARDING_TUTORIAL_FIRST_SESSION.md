# CA40 - Onboarding Tutorial First Session

Plan: CA40

Branch: `codex/CA40-onboarding-tutorial-first-session`

## Summary

CA40 adds a first-session tutorial surface for the graphical GPU alpha. The new
panel guides a new player through observing the selected creature, using
pause/run/step/follow controls, reading food and hazard markers, and checking
GPU/fallback state.

The tutorial is display-only. It does not emit actions, mutate cognition, write
weights, bypass arbitration, or change simulation semantics.

## Files Changed

- `crates/alife_game_app/src/onboarding_tutorial.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `docs/creatures_agi_roadmap_pack/status/CA40_ONBOARDING_TUTORIAL_FIRST_SESSION.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

## Runtime Code Changed

Yes. CA40 adds:

- a Bevy-free onboarding tutorial summary/smoke contract;
- a visible `First Steps` Bevy overlay panel;
- a CLI smoke command:
  `cargo run -p alife_game_app --bin alife_game_app -- onboarding-tutorial-smoke crates/alife_world/tests/fixtures/gpu_alpha`.

## Core APIs Changed

No. `alife_core` is unchanged.

## Public APIs Changed

The `alife_game_app` CLI now includes `onboarding-tutorial-smoke <p34-fixture-root>`.

## Tests Added/Changed

- `ca40_onboarding_tutorial_smoke_guides_first_gpu_alpha_session`
- `bevy_feature_ca40_first_session_tutorial_panel_is_visible_and_bounded`

## Focused Evidence

The CA40 smoke verifies:

- checklist items exist for observe, pause/run/step, follow, food/hazard, and GPU/fallback;
- graphical controls are verified through the deterministic controls smoke;
- food and hazard markers are present in the GPU alpha fixture;
- tutorial text uses stable IDs only;
- tutorial is display-only and has no action or weight authority;
- CPU shadow gate and `full_auth=false` boundary remain visible.

## Invariant Checks

- No S12/G25/P37 created.
- No release tag created.
- No screenshots/logs/target/model/cache artifacts tracked.
- No Bevy/wgpu/GPU/model-runtime dependency added to `alife_core`.
- CPU fallback preserved.
- CPU shadow parity wording preserved.
- No full action-authoritative GPU claim.
- Bevy remains presentation-only.
- Stable IDs only in player-facing tutorial text.

## Known Limitations

- The tutorial is a compact first-session guide, not a full interactive quest
  system.
- Completion state is inferred from existing runtime panel and telemetry; it
  does not persist tutorial progress.
- Keyboard/mouse UX still relies on existing graphical controls and smoke paths.

## Validation Results

Recorded in the CA40 receipt after focused and full validation.

## Next Plan

CAR40 - Polish and tutorial review.
