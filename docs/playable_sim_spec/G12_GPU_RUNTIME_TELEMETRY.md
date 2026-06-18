# G12 GPU Runtime Telemetry

G12 exposes GPU runtime selection as a product-safe option in the playable app.
The default path remains CPU reference and does not require GPU hardware.

## CI-safe smoke

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
```

This command must pass without GPU hardware. It reports CPU fallback status,
no-readback policy, and the manual hardware command. CPU fallback is not GPU
performance.

## Feature-gated product bridge

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-product-smoke
```

This exercises the app bridge to the P29 GPU runtime contracts without creating
a GPU device or requiring hardware in CI.

## Manual GPU hardware report

Run this only on a machine where GPU runtime support is intentionally enabled
and validation has passed:

```powershell
$env:ALIFE_GPU_RUNTIME_BACKEND = 'static'
$env:ALIFE_GPU_RUNTIME_FEATURE = '1'
$env:ALIFE_GPU_RUNTIME_AVAILABLE = '1'
$env:ALIFE_GPU_RUNTIME_VALIDATED = '1'
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Single-line shell form used by app manifests and docs:

```text
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

If any hardware or validation flag is absent, the report may honestly record
CPU fallback rather than GPU performance. Do not convert CPU fallback data into
GPU FPS claims.

## Report template

```markdown
# G12 GPU hardware report

- Date:
- Machine:
- GPU:
- Driver:
- Feature flags:
- Backend requested:
- Backend selected:
- Fallback reason:
- Static parity:
- Plasticity parity:
- Routing/mask parity:
- Recompaction/swap diagnostics:
- No active gameplay readback:
- Tier 1:
- Tier 10:
- Tier 50:
- Tier 100:
- Tier 250:
- Tier 500:
- Bottlenecks:
- Release-impacting limitations:
```

## Product overlay fields

- backend requested
- backend selected
- fallback reason
- CPU oracle authoritative
- no active gameplay readback
- tick/frame telemetry boundary
- skipped supertiles/tiles where available
- measured GPU performance: true only with hardware timing evidence
