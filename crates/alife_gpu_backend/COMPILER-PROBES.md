# Standalone compiler diagnostics

Preparation only: do not run PC/hardware probes while the existing DX12 process
or its outcome remains unreconciled. Publication of this source is not a task
launch or hardware approval. These tests never load Current, change learned state,
run a trainer, disable production sleep, or qualify a repair.

This diagnostic branches from experiment
`8dbfe72f75b7f0c2e3d180751c7978d604f5a1b0`; its production shader/runtime are
unchanged. Verify the exact published diagnostic commit/tree/patch hash from the
owner's receipt, use a new detached worktree and fresh target/output directories,
and preserve all previous held branches. No PR or merge is part of this work.

## Matrix

The eight exact `ALIFE_COMPILER_PROBE_VARIANT` values are:

| Without helpers | With common ABI/helpers | Body |
| --- | --- | --- |
| `standalone-empty` | `abi-empty` | Empty entrypoint |
| `standalone-barrier` | `abi-barrier` | One storage barrier |
| `standalone-conditional-atomic` | `abi-conditional-atomic` | Header flag rejection, atomic payload, barrier, atomic PREPARED |
| `standalone-converged-atomic` | `abi-converged-atomic` | Equivalent writes through an unconditional barrier |

Standalone empty/barrier sources have no ABI at all. Atomic standalone sources
contain only the actual binding4 header and binding6 atomic-state declarations.
ABI variants use the same body plus the feature-composed common ABI. Every probe
uses the production seven-buffer layout/features/limits, workgroup size1 and an
explicit entrypoint. No validation checks are disabled.

## Rerun after the artifact-capture failure

The recovered SDK-DXC probe reached shader emissions but the old logger failed
on a second `runtime-hal.hlsl`. That exit is an instrumentation failure, not a
target-compiler result. Preserve that output directory. Use the new published
source receipt and a different, nonexistent absolute output directory. Retain the
same admitted process-local SDK search path; do not install or replace anything.

Under the sole owner's existing process/resource guard and outer deadline, this
is the exact test-profile create-only probe (compile separately with --no-run):

```powershell
$env:ALIFE_COMPILER_PROBE_VARIANT = 'standalone-empty'
$env:ALIFE_COMPILER_PROBE_PHASE = 'create'
$env:ALIFE_COMPILER_PROBE_DX12_COMPILER = 'dxc'
$env:ALIFE_COMPILER_PROBE_ARTIFACT_DIR = $NewNonexistentAbsoluteOutputDirectory
cargo test --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_dx12_selected_standalone_probe -- --ignored --exact --nocapture --test-threads=1
```

Require one passed test, pipeline_ok, actual NVIDIA/DX12 identity, and the target
entrypoint among indexed HLSL emissions. Verify every indexed file's size/hash
and preserve raw stdout/stderr/exit plus actual loaded SDK DLL identity. Internal
emissions and sequence numbers alone do not establish target compilation success.
Release and full-Current qualification remain the separate gates documented in
DXC-QUALIFICATION.md; this command does not start training.

These are compiler reproducers, not complete sleep validators. The conditional
and converged bodies have identical outputs for the bounded, nonaliasing supplied
fixtures: flag0 copies header word1 into state word0 then publishes256 in word3;
other flags preserve all state. Empty/barrier bodies preserve all state. The
software oracle checks every one of1024words after each of9 flag/payload cases,
for each variant (72dispatches). Other buffers are never written by these bodies.

## CPU-only validation and durable offline export

Build with the known toolchain/profile and record `rustc -vV`, `cargo -V` and the
small relevant build configuration. Do not dump the environment. All commands are
from this exact checkout:

```sh
cargo test --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::standalone_sources_validate_and_emit_with_explicit_options -- --exact --nocapture
```

Durable export requires `ALIFE_COMPILER_PROBE_ARTIFACT_DIR` to name a new absolute
directory whose parent exists. The test creates it and refuses overwrite:

```sh
cargo test --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::export_standalone_probe_artifacts -- --ignored --exact --nocapture
```

For each variant it writes WGSL, binary SPIR-V, HLSL for SM5.1/6.0, and exact
emitter option/translated-entry records. SHA256 and size of each file are logged.
Naga29.0.3 validates every module and applies the same process_overrides entrypoint
selection path as wgpu-hal. Offline register mapping is explicitly binding=register,
space0; offline SPIR-V explicitly uses ReadZeroSkipWrite for all bounds policies,
HLSL restricts indexing, and workgroup zero initialization and loop bounding stay on.
These outputs are reproducible offline inputs, not a claim that an unknown device
selected those register allocations, shader models or SPIR-V capabilities.

With a known CPU-only llvmpipe ICD forced, and the artifact variable UNSET:

```sh
cargo test --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::software_vulkan_standalone_probes_match_cpu_oracle -- --ignored --exact --nocapture --test-threads=1
```

This explicitly selects a CPU Vulkan adapter, never a physical GPU. It is not RTX
or DX12 qualification. Keep normal production CPU rejection unchanged.

## Source-bound hardware contract, held pending reconciliation

Only the sole existing PC owner may run these after its existing process,
activity, consent, resource and safe-window guards admit the work. One selected
variant, backend/compiler and phase per process. Fresh artifact directory each
time; preserve raw stdout/stderr, exit code, actual GPU/driver and artifact hashes.
Record exact source HEAD/tree, clean status and executable/build identity. Compile
first using `--no-run`; enforce an OUTER owned-process deadline (90seconds per
probe after build). Thirty-second device polls do not bound synchronous compiler,
submission or error-scope waits. A timeout is a timeout, not proof of OOM.

Set these explicitly in the owned process (PowerShell example, paths supplied
from verified PC-local facts; save/restore environment values in try/finally):

```powershell
$env:ALIFE_COMPILER_PROBE_VARIANT = 'standalone-barrier'
$env:ALIFE_COMPILER_PROBE_PHASE = 'create' # or 'submit' only for a separately admitted first-dispatch test
$env:ALIFE_COMPILER_PROBE_ARTIFACT_DIR = $NewNonexistentAbsoluteOutputDirectory
```

Vulkan test (strict NVIDIA discrete Vulkan, no fallback):

```powershell
cargo test --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_vulkan_selected_standalone_probe -- --ignored --exact --nocapture --test-threads=1
```

DX12 also requires an explicit compiler choice, with no Auto or fallback:

```powershell
$env:ALIFE_COMPILER_PROBE_DX12_COMPILER = 'dxc' # or separately selected 'fxc'
cargo test --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_dx12_selected_standalone_probe -- --ignored --exact --nocapture --test-threads=1
```

Do not install a compiler/driver or change a DLL search path for these probes.
DXC requests the already resolvable `dxcompiler.dll`; absence fails rather than
falling back. wgpu's normal default is Auto, not necessarily FXC. The current
constructor does not call `.with_env()`, so `WGPU_DX12_COMPILER` alone would not
select these tests' compiler; they set the descriptor explicitly.

Require exactly1test and no failures. In create phase expect pipeline_ok; this is
not first-dispatch success. Submit phase checks9cases×all1024statewords. Actual
adapter/backend/descriptor flags and requested compiler/options are logged. The
logger also captures wgpu-hal's actual FXC/DXC choice and complete generated HLSL.
Every emission, including internal shaders and repeated entrypoint names, gets a
distinct `runtime-hal-NNNNNN-LABEL.hlsl` file. `runtime-hlsl-index.jsonl` binds its
sequence, original entrypoint/stage label, filename, byte count and SHA256. No input
is overwritten. The logger also records HRESULT messages such as Compile/GetOutput
from the DXGI result helper. `runtime.wgsl`, adapter/driver information,
request/options, HAL log and stage log
are preserved in the new directory. HLSL emission does not prove compiler success.
No logger is installed in production; these tests own their process's logger.

The public API does not expose actual HAL SPIR-V, resolved DLL path/version/hash,
all device-selected HLSL options/target or final compiler flags. Offline outputs
are labeled accordingly. The owner must obtain loaded-compiler identity and exact
arguments before claiming a standalone DXC command reproduces the same input.
Debug/optimization choices depend on instance DEBUG and actual device workarounds.
No DXC/FXC compiler was available in this Linux executor; none was downloaded.

## Interpretation

First compare standalone empty/barrier with ABI-included counterparts; the helper
set can materially enlarge HLSL even with a tiny entrypoint. Then compare the
minimal conditional/converged pair if relevant. Do not assume a production cause
from reduced programs that optimize differently.

A DXC Compile HRESULT/diagnostic establishes its reported failure stage; a generic
OutOfMemory receipt does not identify the caller. If exact emitted HLSL separately
fails with the same compiler/target/flags before any D3D12 pipeline creation, that
isolates a compiler-stage reproducer independent of GPU execution. If compilation
succeeds but native pipeline creation fails, examine that later stage. Vulkan NVVM
and DXC outcomes are separate facts. Do not infer driver causation, train from a
probe pass, or publish a production repair without the actual unresolved log and
appropriate full semantic/hardware qualification.
