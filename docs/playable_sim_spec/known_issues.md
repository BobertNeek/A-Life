# Playable Sim Known Issues

This file records non-blocking known limitations surfaced by G22. It is not a
place to hide release blockers. Any release blocker must include exact
reproduction steps and remain marked as a blocker until fixed.

## Current release blockers

None known after the G22 QA smoke and full validation set pass.

## Non-blocking limitations

### Scripted hazard contact in fast balance smoke

Severity: known limitation  
Reproduction:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
```

The fast balance smoke intentionally includes a scripted hazard contact so pain
and avoidance metrics stay visible. This is balance evidence, not proof of a
complete emergent ecosystem.

### Extended balance run is manual

Severity: known limitation  
Reproduction:

```powershell
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
```

The extended run is ignored by default to keep CI bounded. Use it for deeper
local balance inspection.

### GPU hardware performance remains manual unless measured

Severity: manual evidence required  
Reproduction:

```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

If hardware support or validation flags are missing, reports may honestly record
CPU fallback rather than GPU performance. CPU fallback is not a GPU performance claim.

### Graphical playground smoke depends on local graphics support

Severity: manual evidence required  
Reproduction:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

The headless path is the default CI-safe route. Graphical smoke remains manual
and feature-gated.
