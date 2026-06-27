# CA28 - Topological Concept Overlay

Status: implemented on `codex/CA28-topological-concept-overlay`.

## Summary

CA28 adds a read-only topological concept overlay to the graphical alpha app.
The overlay mirrors `alife_core` concept cells, edges, unresolved gaps, and
recent sealed behavior events into bounded UI text. It is presentation-only:
topology remains a bias/context ledger and cannot emit actions, bypass P09
arbitration, mutate cognition, or rewrite weights.

## Player/Developer Surface

- Graphical app panel: `Concept Map (read-only)`.
- Compact status line: concept node, edge, and gap counts.
- Event link: latest tick/sequence/action/target/topology update summary.
- Boundary copy: `bias/context only; no actions`.
- Stable IDs are used in text; Bevy `Entity` IDs are not exposed.

## Focused Evidence

```powershell
cargo run -p alife_game_app --bin alife_game_app -- topological-concept-overlay-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

The smoke validates:

- topology snapshot schema/version,
- concept node rows,
- edge rows,
- behavior-event links,
- player-facing overlay text,
- action-bypass blocked,
- direct cognition mutation blocked.

Graphical smoke remains the product-facing visual evidence command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback remains explicit:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Boundaries

- No `alife_core` dependency change.
- No Bevy/wgpu/model-runtime dependency enters `alife_core`.
- No topology output can issue actions.
- No semantic/SLM authority changes.
- CPU fallback and CPU shadow parity remain unchanged.
- Product GPU claim is unchanged; this plan does not prove full
  action-authoritative GPU runtime.

## Known Limitations

- The overlay is a compact text visualization, not a full interactive graph
  editor.
- Concept labels are summarized from bounded object/action/agent/word bindings.
- Graphical layout evidence remains local machine evidence and should be
  rechecked during the CAR31 cognition inspection review.

## Next Plan

CA29 - Creature memory/history journal.
