# CA03 Visible Intent Feedback

Status: CA03 complete.

## Scope

CA03 makes creature intent and interaction feedback legible in the GPU alpha
playground. The change is presentation-only: it mirrors stable-ID runtime panel
state in the Bevy scene and does not change action semantics, arbitration,
sealed patch handling, GPU authority, or `alife_core`.

## Changes

- The runtime status overlay now shows:
  - a stable-ID intent marker summary,
  - compact action badge labels such as `EAT`, `APPROACH`, `FLEE`, `INSPECT`,
    `SLEEP`,
  - a bounded `Events (last 5)` feed.
- The Bevy graphical scene now includes presentation-only CA03 components:
  - `GraphicalIntentLine`, a stable-ID line from creature `stable:1` to the
    current target when one exists,
  - `GraphicalActionBadge`, a selected action badge anchored over the creature.
- Food and hazard interaction cues remain display-only. Food targets highlight
  when selected, hazard-directed movement is labeled `FLEE`, and hazard markers
  remain visually distinct.
- Tests assert that the event feed remains bounded, player-facing, stable-ID
  safe, and does not expose Bevy `Entity` text.

## Evidence

Focused runtime overlay regression:

```powershell
cargo test -p alife_game_app --test app_shell graphical_runtime_overlay_is_gpu_first_without_false_pretick_events -- --nocapture
cargo test -p alife_game_app --test app_shell graphical_runtime_event_feed_keeps_last_five_meaningful_events -- --nocapture
```

Feature-gated Bevy presentation regression:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell bevy_feature_ca03_intent_line_and_action_badge_are_stable_id_presentation_only -- --nocapture
```

Graphical evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback evidence:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Boundaries

- Bevy visuals remain presentation-only mirrors of runtime panel state.
- Stable IDs are used in player-facing text; Bevy entity IDs remain local only.
- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU shadow parity remains the GPU proposal gate.
- No full action-authoritative GPU runtime is claimed.
- CA03 food/hazard cue labels are tied to the deterministic GPU alpha fixture's
  stable IDs (`stable:2` food and `stable:3` hazard); broader kind-driven
  targeting remains later UI/runtime work.
- No screenshots, logs, target artifacts, release tags, S12, G25, or P37 were
  created.

## Remaining Work

- CA04 owns reset/terminal-state recovery and alpha run-loop stability.
- Later UI plans own broader panel layout, richer camera/mouse controls, and
  deeper creature state visualization.
