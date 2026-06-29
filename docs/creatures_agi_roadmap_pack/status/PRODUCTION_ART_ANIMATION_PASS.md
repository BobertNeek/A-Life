# Production Art Animation Pass

Goal: move the committed A-Life `alpha_art_v1` pack beyond static replacements by adding production-alpha creature pose frames and live Player View art selection, without changing simulation semantics.

Branch: `codex/production-art-animation-pass`
Status: implemented and validated on the feature branch

## Visual Direction

This pass follows the generated local blueprint for an original top-down organic ecosystem art sheet. The blueprint is untracked reference material only. Product assets remain deterministic committed PNGs produced by:

```powershell
python scripts/generate_alpha_art_v1.py
```

## Assets Added

New committed PNG assets under `crates/alife_game_app/assets/alpha_art_v1/`:

- `creature_moving.png`
- `creature_eat.png`
- `creature_sleep.png`
- `creature_signal.png`
- `selection_pulse.png`
- `food_bloom.png`
- `hazard_glow.png`

The pack now contains 22 PNG entries. All are original project-generated assets, 128x128, below the 64 KB per-file cap, and listed in `alpha_art_manifest.json`.

## Manifest And Validation Changes

The alpha art manifest art direction is now:

```text
production-alpha-organic-topdown-v3
```

Required manifest roles now include:

- creature idle, hurt, moving, eat, sleep, and signal frames
- selection ring and selection pulse
- food and food variant
- hazard and active hazard variant
- rock/obstacle
- primary terrain tiles
- prop/dressing variants

Manifest validation still checks schema/version, required roles, PNG header, dimensions, file size, malformed PNG rejection, missing role rejection, and forbidden artifact paths.

## Rendering Changes

Default Player View still uses asset-backed sprites for creature, food, hazard, obstacle, selection, terrain, and props. This pass adds display-only art selection for existing CA38 creature poses:

- `move-lean` -> `creature_moving.png`
- `eat-reach` -> `creature_eat.png`
- `sleep-curl` / `rest-low` -> `creature_sleep.png`
- `social-signal` / `inspect-focus` / `curious-tilt` -> `creature_signal.png`
- `pain-flinch` / `flee-alert` -> `creature_hurt.png`
- idle/default -> `creature_idle.png`

Food targets can bloom with `food_bloom.png`. Hazards can use the active `hazard_glow.png` sprite. Selected creatures also receive an asset-backed `selection_pulse.png`.

These are presentation-only mappings from existing runtime summaries. They do not emit actions, mutate cognition, change physics, alter persistence, or make Bevy authoritative.

## Focused Evidence

Focused commands run:

```powershell
python scripts/generate_alpha_art_v1.py
cargo fmt --all -- --check
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs -- --nocapture
cargo test -p alife_game_app ca12_app_bundle_manifest_discovers_assets_shaders_and_placeholder_art -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_alpha_art_pose_mapping_uses_committed_animation_frames -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results: PASS. Two initial Bevy focused commands timed out while waiting on build locks during parallel execution, then passed sequentially with a longer timeout.

The 30-second graphical GPU smoke selected `GpuPlastic` and exited cleanly with Player View acceptance true. The forced fallback smoke selected `CpuReference`, reported `HardwareUnavailable`, showed degraded fallback state, and exited cleanly.

## Full Validation

Commands run:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

Results: PASS. An earlier all-features workspace test run hit a Windows `STATUS_ACCESS_VIOLATION` in the captured `app_shell` harness; the isolated captured `app_shell` all-features test and the exact full all-features workspace command both passed on rerun without code changes.

## Known Limitations

- This is still a small alpha sprite pack, not a final commercial art set.
- Creature animation is frame/pose selection plus existing transform/pulse behavior, not a full multi-frame animation sheet.
- The art direction still needs broader biome cohesion, UI skinning, lighting/VFX polish, and live screenshot review against player expectations.

## Invariant Checks

- No CA45 work started.
- No release tag created.
- No S12, G25, or P37 created.
- No `alife_core` dependency changes.
- No action authority changes.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No semantic/SLM authority changes.
- No neural compression, custom sensory raycasting, planet topology, or ExperiencePatch transaction work.

## Artifacts

Tracked: source code, docs, manifest, and versioned product PNG assets under `crates/alife_game_app/assets/alpha_art_v1/`.

Untracked/forbidden: screenshots, logs, target artifacts, model files, caches, temporary generator outputs, and generated reference images.

## Next Work

Continue production art improvement with broader biome composition, stronger UI skinning, and visual review. The active goal is not complete yet.
