# CA23 - Graphical School Mode And Lesson Panel

Status: complete on branch `codex/CA23-graphical-school-mode-lesson-panel`.

## Summary

CA23 adds a player-facing graphical school panel for the existing G10 school
mode evidence. The panel is display-only and uses the already validated sealed
patch verifier path rather than introducing model inference or teacher action
authority.

## Player-Facing Surface

- School mode is visible as a compact `School Mode` panel in the graphical app.
- `T` toggles graphical school mode on/off. When off, teacher cue markers are
  hidden and the panel records that sealed-patch verifier evidence remains
  available without granting teacher action authority.
- A presentation-only `[T] teacher` marker and lesson cue marker are spawned in
  the Bevy scene using stable IDs only.
- The panel shows teacher stable ID, learner stable ID, current lesson,
  completed step count, teacher cue markers, sealed patch verifier status, and
  the perception-only boundary.

## Boundary Evidence

- Teacher cues enter through perception channels.
- The verifier reads sealed patches.
- Teacher metadata does not bypass normal action arbitration.
- No hidden vector injection is introduced.
- The panel is display-only and cannot emit actions or rewrite weights.

## Commands

Focused smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- graphical-school-mode-smoke
```

Graphical smoke remains:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback graphical smoke remains:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Known Limitations

CA23 does not use a real semantic provider or SLM. CA26 owns the real local
embedding provider, and CA27 owns the real local SLM prior. CA23 does not add
teacher speech tokens beyond the existing G10 perception-cue evidence; CA24
owns richer teacher gestures and token objects.
