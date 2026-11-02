# Explicit DXC qualification after FXC X3705

On the flat-validation experiment branch, use FLAT-SLEEP-VALIDATION.md as the
authoritative candidate contract. The historical explicit-DXC change below
preserved shader bytes; the later experiment changes initializer control flow
and adds explicit FXC selection. Its source and hardware gates are separate.


Recovered PC evidence: RTX 3050, DX12 driver 32.0.16.1088,
initialize_sleep_transaction pipeline creation failed with FXC X3705: flattening an
if because of nesting limits encountered side effects. This is not DXC OOM.
Do not repeat the unchanged Auto/FXC path as a new candidate.

This change preserves every shader byte from 8ae7b506, including all predicates,
mutable snapshots, atomics and barriers. It adds a strict opt-in compiler request:

```powershell
$env:ALIFE_GPU_BACKEND = 'dx12'
$env:ALIFE_GPU_DX12_COMPILER = 'dxc'
```

Both are required for the production factory. Missing compiler setting preserves
old Auto behavior; explicit dxc sets DynamicDxc(dxcompiler.dll), with no FXC
fallback. Invalid values and a compiler request without strict DX12 selection are
refused. Do not install a DLL, modify PATH or replace a driver as part of this
packet. Missing/unusable DXC is a blocker to report. WGPU_DX12_COMPILER is not the
application setting and is not consumed by its current descriptor constructor.

## Sole PC owner contract

Use the exact published source commit/tree/patch hash from the task receipt in a
new detached worktree and fresh target/output folders. Record clean source state,
rustc/cargo, relevant build flags, actual adapter/backend/driver and binary hashes.
The existing owner must first reconcile prior processes, admit resources/consent
and enforce a bounded outer deadline. Do not close unrelated applications, reset
the device or clear caches. No real Current is loaded by these synthetic tests.
Save/restore environment values in try/finally. One test process at a time.

The existing standalone diagnostic can first check already-installed DXC. Select
standalone-empty, create phase, explicit dxc and a NEW artifact directory using
ALIFE_COMPILER_PROBE_VARIANT, ALIFE_COMPILER_PROBE_PHASE,
ALIFE_COMPILER_PROBE_DX12_COMPILER and ALIFE_COMPILER_PROBE_ARTIFACT_DIR. Then:

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::standalone_compiler_probes::nvidia_dx12_selected_standalone_probe -- --ignored --exact --nocapture --test-threads=1
```

That captures actual compiler trace/HAL HLSL/adapter. Before further commands,
unset those four diagnostic variables while retaining the two ALIFE_GPU variables
above. Production qualification uses the new shared compiler-request parser.
Do not infer full initializer success from the standalone-empty result.

Use release qualification to match the native release build's debug-assertion
bucket; preserve any extra actual build flags and report profile differences.
Compile all targets separately with --no-run before enforcing run deadlines.
Every exact filtered command must execute 1 test, never 0. Preserve raw stdout,
stderr, exit status and failures. Suggested outer limits after build: 180 seconds
per initializer/oracle/kernel/R12 test, 300 seconds per full sleep/checkpoint target.
In-harness poll timeouts do not bound synchronous compilation or scope waits.

### Full initializer and unchanged original oracle

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::barrier_diagnostics::nvidia_dx12_converged -- --ignored --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test sleep_initializer_equivalence device_oracle::nvidia_dx12_initializer_matches_frozen_legacy_for_all_corruptions -- --ignored --exact --nocapture --test-threads=1
```

The oracle requires NVIDIA discrete DX12 and the shared explicit compiler request.
It compares all 512 state words for 8,535 cases against the frozen b9f78ed0 original,
checks positive completion payloads/status, and must detect the weakened-bank
negative control. Frozen output must also match the independent CPU Vulkan digest
(u64 order): db70c676c01908bb cbe91726a0435cd3 02dcbc58f3e69598 ad2b2bc0771fa666.
A mismatch or original/candidate compilation failure blocks qualification. Never
refresh the golden to fit DXC. This tests initializer semantics, not all training
numerics.

### Full production kernels and synthetic acceptance

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::kernel_diagnostics::nvidia_dx12_each_production_kernel_compiles_and_dispatches -- --ignored --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_fast_plasticity r12_native_success_promotes_exactly_one_transaction -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_runtime hardware::hardware_receipt_is_one_bounded_required_backend_contract -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_sleep -- --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_checkpoint -- --nocapture --test-threads=1
```

Require exactly 28 named kernel create/submit successes inside the 1 kernel test,
R12=1, receipt=1, sleep=7 and checkpoint=6 passed, all without failures. Inspect
alife_requested_dx12_compiler=DynamicDxc and actual RTX/DX12 hardware receipts.
The explicit compiler configuration cannot fall back to FXC. The wrapper only
shares compiler admission, so BOTH production environment settings remain required.
Bind the real loaded dxcompiler.dll path/version/hash and effective compile flags
from PC evidence; a request string does not establish DLL identity. Do not claim
Vulkan/NVVM success from DX12 results.

## Matching native build and care continuation

Only after the route qualifies, build both native tools from this exact source:

```powershell
cargo build --release --locked -p alife_game_app --features foundation-training --bin train_n2048_care --bin export_native_training_handoff
```

Record both executable hashes, --build-info/--resident-contract and actual backend
receipt. Keep the same admitted DXC DLL identity and environment on launch and
recovery. The updated continuous-care helper pins dx12_compiler_selection in
owner state and final receipt independently of semantic_prior. Its pin is the
request string; PC ownership must separately bind the loaded DLL identity.

Use the existing full-Current admission contract on a disposable copy first,
including full actor/value weights, Adam m/v/u32 ages and masks/config,
private world/recurrent/sleep state, RNG and exact pending seed tail. No rollback,
fresh world, b817 source substitution or accepted-Current mutation during probes.
Supply existing admitted PC store/owner paths and verified binary hashes to the
existing run_continuous_care.py continuation; do not invent paths or reservations.
Keep semantic_prior on/deterministic and acquired identity unchanged. Preserve
resource and owner leases, capture/receipt/publication/cleanup requirements.
For 2026-11-02, --night-date 2026-11-02 derives a 14:57 UTC collection cutoff and 15:00 UTC
hard end with a 180 second reserve. A qualified backend alone does not bypass full
state admission or authorize training beyond that cutoff. No PR/merge is included.
