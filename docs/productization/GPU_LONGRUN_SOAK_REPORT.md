# GPU Long-Run Soak Report

Status: manual local evidence captured for the existing combined GPU
static/plastic CPU-shadow-guarded runtime.

## Command

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 1000 --report-every 100
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 5000 --report-every 500
```

The command is manual evidence. It does not change the CI smoke cap for
`full-gpu-runtime-smoke`, which remains bounded at 16 ticks.

## Local Hardware

- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Driver: 581.80
- Selected runtime backend: `GpuPlastic`
- CPU fallback: still available through `ALIFE_GPU_RUNTIME_AVAILABLE=0`

## 1000-Tick Result

- Requested ticks: `1000`
- Completed ticks: `1000`
- Sealed patches: `32`
- Packed logs: `32`
- GPU static dispatch ticks: `1000`
- GPU proposal ticks: `1000`
- CPU shadow parity checks: `1000`
- Parity failures: `0`
- First parity failure tick: none
- H_shadow applications: `1`
- H_shadow rejections: `0`
- H_shadow records applied: `2`
- Max H_shadow absolute delta: `0.112549`
- `W_genetic_fixed` unchanged: `true`
- lifetime-consolidated unchanged: `true`
- H_operational unchanged: `true`
- Compact active readback: `64000` bytes total
- Post-seal H_shadow diagnostic readback: `48` bytes total
- Wall time: `28941.1680 ms`
- Average: `28.9412 ms/tick`
- Throughput: `34.55 ticks/sec`

## 5000-Tick Result

- Requested ticks: `5000`
- Completed ticks: `5000`
- Sealed patches: `32`
- Packed logs: `32`
- GPU static dispatch ticks: `5000`
- GPU proposal ticks: `5000`
- CPU shadow parity checks: `5000`
- Parity failures: `0`
- First parity failure tick: none
- H_shadow applications: `1`
- H_shadow rejections: `0`
- H_shadow records applied: `2`
- Max H_shadow absolute delta: `0.112549`
- `W_genetic_fixed` unchanged: `true`
- lifetime-consolidated unchanged: `true`
- H_operational unchanged: `true`
- Compact active readback: `320000` bytes total
- Post-seal H_shadow diagnostic readback: `48` bytes total
- Wall time: `142803.4531 ms`
- Average: `28.5607 ms/tick`
- Throughput: `35.01 ticks/sec`

Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`.

Full action-authoritative claim: `false`.

## Forced Fallback

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 100
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result:

- Selected backend: `CpuReference`
- Fallback reason: `HardwareUnavailable`
- Completed ticks: `100`
- Sealed patches: `32`
- GPU static dispatch ticks: `0`
- GPU proposal ticks: `0`
- H_shadow applications: `0`
- Product runtime claim: `None`

## Interpretation

The long-run soak shows that the combined GPU static scoring plus post-seal
H_shadow application path remains stable for 5000 manual ticks on the local RTX
3050/Vulkan adapter. CPU shadow parity was checked every tick and no parity
failure occurred. Active readback remained bounded to compact action summaries;
the H_shadow readback was post-seal diagnostic evidence.

The manual run does not prove full action-authoritative GPU runtime. CPU shadow
parity remains a runtime gate before GPU proposal scores are used.

## Known Limitations

- The fixture stops producing new sealed patches after 32 patches in this
  bounded headless scenario, so the soak primarily validates repeated GPU static
  scoring/parity over later ticks plus one safe post-seal H_shadow application.
- H_shadow application is intentionally attempted once for this fixture because
  the diagnostic plasticity records include before-values and the core contract
  correctly rejects replaying the same deltas after live H_shadow state changes.
- Timing is local hardware evidence only and is not a release-wide performance
  claim.
- No release tag was created.

