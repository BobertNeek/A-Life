# GPU Sustained-Learning Soak Report

Status: manual local evidence path added for repeated valid post-seal H_shadow
applications on the existing combined GPU static/plastic CPU-shadow-guarded
runtime.

## Command

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 1000 --report-every 100
```

The command is manual evidence. It does not change the CI-safe cap for
`full-gpu-runtime-smoke`, and it does not replace the existing
`gpu-longrun-soak` stability command.

## Method

The previous `gpu-longrun-soak` proved 5000 GPU-scored ticks, 5000 CPU shadow
parity checks, and zero parity failures, but the bounded P34 fixture saturated
its stored sealed-patch/log evidence at 32 records. It also applied H_shadow
once because replaying the same diagnostic H_shadow before-values after live
state changes is correctly rejected by the post-seal core contract.

The sustained-learning soak keeps the same GPU semantics and uses deterministic
episode rotation:

1. Run the same combined CPU-shadow-guarded GPU static/plastic path.
2. Aggregate evidence across bounded deterministic episodes.
3. Reinitialize the tiny P34 fixture per episode so the post-seal H_shadow
   delta batch has fresh before-values and fresh sequence evidence.
4. Apply each H_shadow batch through
   `CreatureMind::apply_post_seal_lifetime_deltas`.

This is evidence for repeated valid post-seal H_shadow application across
deterministic episodes. It is not evidence for unbounded single-creature
lifetime learning over one continuous world run.

## Local 100-Tick Bring-Up

Command:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 100 --report-every 25
```

Result:

- Selected backend: `GpuPlastic`
- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Driver: 581.80
- Completed ticks: `100`
- Episodes: `4`
- Episode ticks: `32`
- Sealed patches total: `100`
- Packed logs total: `100`
- GPU static dispatch ticks: `100`
- GPU proposal ticks: `100`
- CPU shadow parity checks: `100`
- Parity failures: `0`
- H_shadow application attempts: `4`
- H_shadow applications succeeded: `4`
- H_shadow applications rejected: `0`
- H_shadow records applied: `8`
- Max H_shadow absolute delta: `0.112549`
- `W_genetic_fixed` unchanged: `true`
- lifetime-consolidated unchanged: `true`
- H_operational unchanged: `true`
- Compact active readback: `6400` bytes total
- Post-seal H_shadow diagnostic readback: `192` bytes total
- Wall time: `4565.3799 ms`
- Average: `45.6538 ms/tick`
- Throughput: `21.90 ticks/sec`
- Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- Full action-authoritative claim: `false`

## 1000-Tick Result

Command:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 1000 --report-every 100
```

Result:

- Selected backend: `GpuPlastic`
- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Driver: 581.80
- Completed ticks: `1000`
- Episodes: `32`
- Sealed patches total: `1000`
- Packed logs total: `1000`
- GPU static dispatch ticks: `1000`
- GPU proposal ticks: `1000`
- CPU shadow parity checks: `1000`
- Parity failures: `0`
- H_shadow application attempts: `32`
- H_shadow applications succeeded: `32`
- H_shadow applications rejected: `0`
- H_shadow records applied: `64`
- Max H_shadow absolute delta: `0.112549`
- Compact active readback: `64000` bytes total
- Post-seal H_shadow diagnostic readback: `1536` bytes total
- Wall time: `30901.3262 ms`
- Average: `30.9013 ms/tick`
- Throughput: `32.36 ticks/sec`
- Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- Full action-authoritative claim: `false`

## 5000-Tick Result

Command:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 5000 --report-every 500
```

Result:

- Selected backend: `GpuPlastic`
- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Driver: 581.80
- Completed ticks: `5000`
- Episodes: `157`
- Sealed patches total: `5000`
- Packed logs total: `5000`
- GPU static dispatch ticks: `5000`
- GPU proposal ticks: `5000`
- CPU shadow parity checks: `5000`
- Parity failures: `0`
- H_shadow application attempts: `157`
- H_shadow applications succeeded: `157`
- H_shadow applications rejected: `0`
- H_shadow records applied: `314`
- Max H_shadow absolute delta: `0.112549`
- Compact active readback: `320000` bytes total
- Post-seal H_shadow diagnostic readback: `7536` bytes total
- Wall time: `152486.9062 ms`
- Average: `30.4974 ms/tick`
- Throughput: `32.79 ticks/sec`
- Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- Full action-authoritative claim: `false`

## Fallback Behavior

Forced fallback remains supported:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 100
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

When forced fallback is active, the report must select `CpuReference`, seal CPU
patches, and make product runtime claim `None`. It must not claim GPU static
dispatch, GPU proposals, or H_shadow GPU applications.

Local 100-tick forced fallback result:

- Selected backend: `CpuReference`
- Fallback reason: `HardwareUnavailable`
- Completed ticks: `100`
- Sealed patches total: `100`
- GPU static dispatch ticks: `0`
- GPU proposal ticks: `0`
- H_shadow applications succeeded: `0`
- Product runtime claim: `None`

## Known Limitations

- Repeated H_shadow application is achieved through deterministic episode
  rotation, not through one unbounded single-creature lifetime run.
- CPU shadow parity remains the gate before GPU proposal scores are used.
- The command does not prove full action-authoritative GPU runtime.
- Timing is local RTX 3050/Vulkan evidence only.
- No release tag was created.
