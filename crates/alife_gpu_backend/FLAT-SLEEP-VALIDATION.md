# Flat sleep validation candidate

Update: the real RTX/DX12/FXC candidate production pipeline passed. The frozen
OLD native kernel then failed FXC X3663 before oracle comparison. Under explicit
authorization, CANDIDATE-CPU-GOLDEN.md now defines the hardware oracle: candidate
and negative-control execution compared word-for-word against pinned original
CPU output. The old CPU comparison remains available; it is not required to
compile the old kernel on the compiler-limited native path.


This is an unqualified compiler-compatibility experiment based on 618a53c6. The
known FXC X3705 failure reported an if that could not be flattened because of
nesting and side effects. Separately, the original DXC input reached native PSO
creation and failed there even with target SetName omitted. Neither fact proves
a root cause for the PSO failure. This candidate changes initializer control flow
while preserving all validation and state behavior; it does not disable sleep.

## Change and semantic evidence

Only initialize_sleep_transaction is rewritten. Seventy-four scalar locals retain
the same ordered snapshot assignments. Each validation stage is guarded by a
private valid flag; first rejection performs its existing write and clears that
flag. Later loads and writes are then skipped. This replaces validation returns
and outer-loop breaks with local joins. The phenotype loop still stops on its
first mismatch. Initial invalid bounds still cause no writes. Completion clear
and identity writes remain before mutable extension/learning snapshots, final
payload writes precede one unconditional storageBarrier, and only valid success
publishes PREPARED. No new dispatch, scratch buffer, ABI or durable state is added.
All other shader functions and entrypoints are byte-identical to 618a53c6.

Independent actual-source review matched every condition, load offset, snapshot
and store against frozen 1bba259e after accounting for the wrappers and private
initialization. All 73 rejection sites remain. The frozen b9f78ed0 CPU oracle passed
all 8,535 cases × 512 state words with the unchanged original-output digest and detected
the weakened-bank negative control. This includes malformed fields, both banks,
completion overlaps, relocated records and concurrent disjoint jobs. Twelve
WGSL/layout/fixed-sleep checks, all 28 software kernels, diagnostic regressions,
compiler admission tests, care-helper tests and strict Clippy also passed.

This is not a size or memory optimization claim: offline declared-option HLSL
increased from 26,974 to 42,013 bytes. It changes continuation structure. Compiler
nesting/resource behavior and native correctness require the real-device gates.
Naga still compacts to one entrypoint/seven helpers; redundant compaction is not
the repair. Canonical feature-composed candidate WGSL is 71,645 bytes, SHA256:
98db1eb206356c29c727a7820831df20158f104ca08cbcc7a66b8efbf3ac010f.
The old 27,123-byte actual DXC HLSL hash 1bec64... is the BASELINE input; this candidate
must produce a new input. Capture its exact actual HLSL and indexed hash.

## Sole PC owner: one candidate route at a time

Use the published candidate commit/tree/patch receipt in a fresh clean detached
worktree and target/output folders. Preserve all previous inputs and failures.
The existing owner must enforce consent/resource/lease/safe-window guards and
reconcile prior processes. Record source/build identity, actual compiler DLL
path/version/hash, release flags and adapter/backend/driver. No DLL installation,
GPU reset, cache clearing, Current load or training occurs in these probes.

Production compiler selection now accepts exact fxc or dxc only with strict DX12;
absence keeps the old Auto default. Both explicit routes fail without fallback.
FXC is the direct falsifier of the documented nesting failure and requires no
SDK-path change; use the already available compiler. Preserve/restore environment
values in try/finally. Build with --no-run first, then apply the owner's outer
180-second deadline to each initializer/oracle/kernel/R12 run, 300 seconds to full
sleep/checkpoint targets. Compile time is separate. One process at a time.

```powershell
$env:ALIFE_GPU_BACKEND = 'dx12'
$env:ALIFE_GPU_DX12_COMPILER = 'fxc'
$env:ALIFE_COMPILER_PROBE_ARTIFACT_DIR = $NewNonexistentAbsoluteOutputDirectory
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_dx12_production_initializer_native_stages -- --ignored --exact --nocapture --test-threads=1
```

Require exactly 1 passed test, actual NVIDIA/DX12 and explicit FXC evidence, target
pipeline_ok and indexed target HLSL. Preserve full compile/nativePSO/HRESULT
records. Any failure falsifies this route in that context; do not claim repair.
The separate DXC route uses dxc instead and the admitted SDK identity, with fresh
output and its own receipt. Do not count one compiler's pass as the other's pass.

## Hardware semantic and production gates after initializer success

Unset the diagnostic artifact-directory variable before these independent tests;
retain both production backend/compiler settings. Every exact filter must execute
one test, never zero. Use the passing route and same release flags/DLL identity.

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test sleep_initializer_equivalence device_oracle::nvidia_dx12_candidate_matches_frozen_cpu_original_for_all_corruptions -- --ignored --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::kernel_diagnostics::nvidia_dx12_each_production_kernel_compiles_and_dispatches -- --ignored --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_fast_plasticity r12_native_success_promotes_exactly_one_transaction -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_runtime hardware::hardware_receipt_is_one_bounded_required_backend_contract -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_sleep -- --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_checkpoint -- --nocapture --test-threads=1
```

Require captured original/candidate full-state equality, unchanged pinned CPU output
digest/raw SHA256 and detected negative control; exactly 28 named kernel create/submit successes;
R12=1, receipt=1, sleep=7 and checkpoint=6 passed. The known old native compilation failure is preserved as evidence; the candidate
must match its independently captured CPU-original outputs without refreshing the
reference or dropping a case/word. See CANDIDATE-CPU-GOLDEN.md. All these fixtures are synthetic. They never admit
or mutate accepted Current and do not establish all training numerical behavior.

## Care continuation remains a separate admission

After hardware gates, build matching train_n2048_care and
export_native_training_handoff from this exact candidate source, using the
existing release/foundation-training build contract. Record executable hashes,
--build-info/--resident-contract and actual backend/compiler receipt. Keep the same
admitted compiler identity. The existing full Current admission must retain
actor/value weights, Adam m/v/u32 ages/masks/config, private world/recurrent/sleep
state, RNG and exact pending seed tail. Qualify a disposable copy first; no reset,
fresh-world fallback, b817 substitution or accepted-state mutation in these probes.

The continuous-care owner pins backend and compiler requests separately from
semantic_prior and carries them into recovery/cleanup/receipts. Do not change
semantic_prior identity or acquired learning. Retain all resource/owner leases,
capture/publication/cleanup obligations and source/binary/hash binding. For
2026-11-02, the existing night contract ends collection at 14:57 UTC with 180 seconds
reserve and hard end at 15:00 UTC. No held branch/PR is published or merged by this
candidate. Native compilation, execution and actual care training remain pending.
