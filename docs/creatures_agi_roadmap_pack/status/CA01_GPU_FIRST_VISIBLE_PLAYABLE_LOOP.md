# CA01 GPU-First Visible Playable Loop

Status: CA01 complete.

## Scope

CA01 reframes the existing graphical alpha first screen as a GPU-first creature
simulation without changing backend semantics. It does not add new gameplay
systems, GPU authority, persistence changes, or `alife_core` dependencies.

## Changes

- The runtime overlay now uses state-aware player event lines.
- Before the first tick, the overlay prompts the player to press Space or N
  instead of claiming that a GPU proposal or sealed patch already happened.
- After a tick, the overlay reports the actual tick status, selected action,
  target stable ID, sealed patch count, and H_shadow learning cue.
- GPU technical details remain visible but are compact and explicitly keep the
  CPU shadow gate and no-full-action-authoritative boundary.

## Evidence

Focused regression:

```powershell
cargo test -p alife_game_app --test app_shell graphical_runtime_overlay_is_gpu_first_without_false_pretick_events -- --nocapture
```

The test verifies:

- first-screen overlay includes the GPU-first launch prompt,
- pre-tick overlay does not claim accepted GPU proposals or sealed patches,
- one step changes the event feed to the real action/tick/patch state,
- player-facing text remains stable-ID based and does not expose Bevy entities.

Graphical evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

This passed with `gpu_selected=GpuPlastic`, `gpu_scores=true`,
`cpu_shadow_parity=true`, `fallback=None`, and the GPU alpha fixture containing
one creature, one food marker, and one hazard marker.

Forced fallback evidence:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

This passed with `gpu_selected=CpuReference`, `fallback=HardwareUnavailable`,
and no GPU performance claim.

## Boundaries

- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU shadow parity remains the gate.
- CPU fallback remains available and explicit.
- Bevy visuals remain presentation-only.
- No full action-authoritative GPU runtime is claimed.
- No screenshots, logs, target artifacts, release tags, S12, G25, or P37 were
  created.

## Remaining Work

- CA02 owns further fixture/content expansion.
- CA03 owns richer visible intent, movement, targeting, and interaction
  feedback.
- CA04 owns reset/terminal-state hardening beyond the existing visible warning.
