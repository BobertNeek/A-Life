# Exact production DX12 stage capture

The recovered release initializer failure reported 0x8007000E. Its producing
component remains unknown. Standalone-empty success does not qualify production
sleep. This diagnostic observes the existing calls, without replacing DXC,
changing compiler flags, pruning the shader, or altering any sleep predicate,
atomic, barrier, checkpoint or Current state.

The source is exactly runtime_shader_source(CLOSED_LOOP_CONSOLIDATE_WGSL), with
production seven-buffer layout, features, limits and compilation options. With
training-rollout its canonical WGSL is 67,207 bytes, SHA256:
9b6c39fe5afc6be738dc538649aef1e64a94b6a63725ba953ad357a7e01ffd5e.
Creation only: no target dispatch or training is performed.

## Sole PC owner: one bounded release diagnostic

Materialize the exact published commit/tree from the task receipt into a new
clean detached worktree. Preserve prior artifacts. Reconcile old processes, apply
the existing resource/consent/safe-window guard, and retain the admitted SDK DLL
search path, compiler version/hash and release build flags. No installation,
DLL replacement, driver change or cache clearing is part of this packet.
Use a new nonexistent absolute artifact directory with an existing parent.
Record source/tree/clean status, rustc/cargo, build flags, executable identity,
loaded DLL path/version/hash and actual adapter/backend/driver. Do not dump env.

```powershell
$env:ALIFE_GPU_BACKEND = 'dx12'
$env:ALIFE_GPU_DX12_COMPILER = 'dxc'
$env:ALIFE_COMPILER_PROBE_ARTIFACT_DIR = $NewNonexistentAbsoluteOutputDirectory
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib --no-run
```

After the CPU build, under the existing owner's OUTER 180-second process deadline:

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_dx12_production_initializer_native_stages -- --ignored --exact --nocapture --test-threads=1
```

Save/restore those process environment values in try/finally. One process, one
exact test; zero tests is invalid. No generic variant/phase settings are needed.
A nonzero exit remains useful diagnostic evidence: preserve raw stdout/stderr and
exit status, not only the hexadecimal code. Timeouts do not establish OOM.

The test explicitly requires NVIDIA discrete DX12 and DynamicDxc. No Auto/FXC/CPU
fallback. It installs the actual-HLSL logger and stage observer before creating
the instance/device, so internal shader emissions are captured too.

## Evidence and interpretation

- runtime.wgsl and runtime-request.txt bind the exact source and request/options.
- runtime-adapter.txt binds adapter/backend/driver.
- runtime-hal-NNNNNN-LABEL.hlsl files preserve every actual HAL-generated input;
  runtime-hlsl-index.jsonl binds original labels, filenames, sizes and SHA256.
- runtime-native-stages.jsonl records existing wgpu-hal profiling scope entry/exit
  for HLSL generation, compile_dxc/compile_fxc and
  ID3D12Device::CreateComputePipelineState. Records carry thread and captured input.
- runtime-hal.log includes HRESULT helper messages with thread, active native
  scope and shader association. runtime-stages.log retains outer API stages.

Match the initialize_sleep_transaction input and its SHA/thread, not an internal
shader. Only actual wgpu_hal DX12 scope targets count as native evidence. HLSL
write events are deliberately unassociated until the new source is emitted.
An input association is consumed after its PSO scope, so later opaque/precompiled
internal pipelines cannot inherit its label. JSONL writes are serialized across
threads; unknown associations remain null. Verify indexed file sizes/hashes.

Scope EXIT is NOT success: it also runs when a fallible call returns an error.
A matching target Compile HRESULT failure localizes that call. A GetOutput
failure localizes output extraction, not necessarily the Compile call. If the
matching target's native PSO scope is entered, shader loading/compilation or its
cache lookup completed; the failure is later than that compiler call. Preserve
native Linkage errors and any SetName error: naming occurs after the PSO scope,
so PSO entry plus a generic later error alone does not prove PSO-call failure.
The full error type/message and trace together identify the producing operation.
Absent or mismatched target events leave the stage unresolved.

## Target SetName exclusion after the matched production failure

The recovered target input was 27,123 bytes, SHA256
1bec64b4e322637e1af58f65d16dca06aa228f1d39e0a15afecbe7fdb0743f99.
DXC and native PSO scopes were entered before the reported 0x8007000E. The next
single diagnostic omits ONLY the target pipeline descriptor label. Shader-module
label, WGSL/HLSL, entrypoint, layout, features/limits, DXC options, constants,
zero-initialization and cache behavior remain unchanged.

With the same strict DX12/DXC settings, admitted SDK DLL/flags, new output folder,
release build and existing owner/deadline guard, run this exact filter instead:

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_dx12_production_initializer_without_pipeline_setname -- --ignored --exact --nocapture --test-threads=1
```

runtime-pipeline-naming.txt records label=None. Core29 preserves that None in the
HAL descriptor; dx12/device.rs skips its conditional SetName. After successful
shader loading, CreateComputePipelineState is then the only remaining fallible
native HRESULT call in that HAL method. Its failure maps Linkage to a core
Internal error; preserve that typed scope error and complete message. Compare the
actual target HLSL hash/size and native scope/thread with the recovered input.
If that identical input again returns a native linkage/Internal 0x8007000E after
PSO entry, target SetName is excluded and the PSO call is identified by the
inspected source path. A mismatch leaves the A/B inference unresolved.

A successful unnamed run shows label-sensitive or context-sensitive behavior;
it does not alone prove SetName caused the earlier failure or qualify care
training. No production labels or shader behavior are changed by this test.

This uses profiling 1.0.18's supported profile-with-tracing feature in dev
builds. HAL's original compiler/native calls and their error propagation are
unchanged. No dependency fork, passthrough shader or unchecked path is used.
Native Windows observation has not run in this cloud executor. Actual DLL
identity and effective DXC arguments remain PC evidence requirements. These
traces observe the same actual selected compiler invocation rather than claiming
that a separately chosen dxc.exe command is identical.

## Pruning assessment: no new repair justified

The production module has six entrypoints and 54 regular helper functions. HAL
already calls Naga29 process_overrides with the selected entrypoint, which runs
compact(KeepUnused::No) before HLSL emission. CPU verification leaves one
entrypoint/seven helpers and removes the unused aggregate sleep validator.
A second compaction produces byte-identical offline HLSL for SM5.1 and SM6.0:
26,974 bytes, SHA256
4950436f1f401e4b8e94b890a3b9cbe472607c534493d866e006477b49b2e556.
The standalone single-entry ABI-helper confound is therefore not evidence that
production still compiles all those helpers. Do not run a redundant compaction
variant as a repair.

Offline production HLSL/options and WGSL are available from the CPU compaction
analysis test when ALIFE_COMPILER_PROBE_ANALYSIS_DIR names a new directory.
Checked offline SPIR-V is explicitly unavailable because Naga reports unsupported
atomic out-of-bounds handling; a text artifact records that limitation. No
unchecked fallback is emitted. This does not change runtime robustness or
establish failure of the actual runtime-generated SPIR-V.

CPU regressions cover compiler-versus-PSO observation, stale-input clearing,
concurrent thread/input identity, all prior small software probes and the exact
production create-only harness. The frozen 8,535-case original/candidate oracle
still checks every 512-word state and its pinned output digest. This diagnostic
is not a qualified production repair. Any subsequent repair must be based on the
actual staged result and pass full semantics/hardware/Current admission gates.
