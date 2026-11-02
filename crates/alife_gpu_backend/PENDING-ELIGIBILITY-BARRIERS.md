# Eligibility finalizer: converged publication barriers

The later CONVERGED-PUBLICATION-BARRIERS.md contract covers the complete barrier
pass. This packet describes historical d139 identities; its golden/corpus remain
unchanged. Its pending oracle is independently runnable after unrelated kernel
failures, subject to process reconciliation. Full module source hashes change
when later finalizers change, even though the pending function stays intact.

The RTX 3050/DX12/FXC 6c188a60 initializer passed the complete unchanged 8,535×512
CPU-original oracle and negative control. The focused probe observed the expected
legacy float >0 discrepancy for a positive subnormal while the integer predicate
matched CPU truth. The next production isolation failed `finalize_pending_eligibility`
with FXC X3663: synchronization in varying flow control. This packet repairs only
that finalizer. It does not repeat DXC or change the qualified initializer source.

The original function prefix, through the last staging store, is moved verbatim
into `prepare_pending_eligibility`; twelve rejection returns now return a zero
result. Success returns the already captured state/selection addresses. The
one-invocation entrypoint calls that helper, executes the first storage barrier,
publishes pending-valid only on success, executes the second storage barrier,
then publishes selection status 3 only on success. There are no entrypoint
returns or conditional barriers. Rejected workgroups gain two empty barriers.
No inter-workgroup synchronization is introduced or assumed. Snapshot/load order,
all guards, staging/atomic store order, alias-sensitive captured addresses,
generation wrap behavior, eligibility/plasticity arithmetic and error handling
remain unchanged. All other shader code is byte-identical to 6c188a60.

## Frozen reference and evidence

`tests/fixtures/pending_eligibility_original_6c188a60.wgsl` freezes the entire old
shader; its SHA256 is
`e83ff58e472c6037c094a3a564215571bc5e9501a6fe20672adb1992c11a8420`.
The new oracle covers 7,947 cases, comparing all 512 mutable-state and 256
frame-payload words per case. Every record field receives bit/boundary corruptions;
additional cases cover diagnostic bits, phenotype lanes, carry/full-u64-wrap,
bounds/overflow, pending/selection overlap, captured-address lifetime, relocated
nonzero brain records and two disjoint/mixed-validity rows. Existing malformed
input behavior is compared to the frozen source, not broadened by this patch.

Explicit CPU Vulkan executes OLD, candidate, weakened family guard, and incorrect
pending-valid publication on separately reset buffers. Full old/candidate equality,
strict captured-golden reread, both negative controls and explicit positive,
rejection/alias/wrap/batch assertions pass. Of 7,947 cases, 5,617 change state;
the guard negative differs in one case and publication negative in 5,617.
Independent review matched the complete staging prefix and publication ordering,
then reconstructed all 6,103,296 output words from the 927-patch/7,947-ID source
fixture plus raw initial stream, byte-identical to captured OLD output.

Pinned corpus SHA256:
`ad21e0cbf8fe7ddc90b522242753cca7af6dde3430459bfeffb5976d767883ba`

Pinned full OLD output SHA256:
`6660fc4ed7e0f439aa185e81fed6d5e3a2918380d54965559a5f9dbb222e43b6`

Oracle composition is ABI + shared activity validation + eligibility body.
Candidate composed WGSL SHA256:
`8db519b7371908ae5dc99aeb8f5fd730e4652b82aa8cd5e6472c79851a934718`.
The production training-rollout composition is checked separately by the 28-kernel
gate; do not confuse an oracle source hash with a captured runtime HLSL hash.

Cloud validation: strict CPU differential/source checks pass, all 28 software
production kernel variants create/submit, 12 CPU WGSL/layout checks and the
prevalidation contract check pass. Five additional hardware admission tests cannot
run on the CPU-only adapter and remain native gates. Independent source/reference
review found no blocker. These are CPU results, not RTX qualification.

## Sole PC owner: exact release FXC gates

Use the published full commit/tree receipt in a clean isolated worktree and fresh
build/output directories. Keep existing owner, consent, process reconciliation,
resource/lease, capture and safe-window guards. No Current, learned checkpoint,
GPU reset, installation, driver change or private-state mutation is part of these
synthetic probes. Preserve/restore process environment in try/finally. Keep both
ALIFE_PENDING_GOLDEN_EXPORT_DIR and ALIFE_SLEEP_GOLDEN_EXPORT_DIR unset. Record
source/build/executable/compiler-DLL hashes and adapter/backend/driver identity.
Build with --no-run separately; apply the owner's 180-second deadline per kernel,
oracle/R12 process, 300 seconds to each full sleep/checkpoint target. One process
at a time. Do not continue after an unsuccessful or uncertain run.

```powershell
$env:ALIFE_GPU_BACKEND='dx12'
$env:ALIFE_GPU_DX12_COMPILER='fxc'
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::kernel_diagnostics::nvidia_dx12_each_production_kernel_compiles_and_dispatches -- --ignored --exact --nocapture --test-threads=1

$env:ALIFE_PENDING_ORACLE_ARTIFACT_DIR=$NewNonexistentAbsoluteOracleDirectory
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test pending_eligibility_equivalence device_oracle::nvidia_dx12_fxc_matches_frozen_cpu_original_all_corruptions -- --ignored --exact --nocapture --test-threads=1
Remove-Item Env:ALIFE_PENDING_ORACLE_ARTIFACT_DIR

cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_fast_plasticity r12_native_success_promotes_exactly_one_transaction -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_runtime hardware::hardware_receipt_is_one_bounded_required_backend_contract -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_sleep -- --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test closed_loop_checkpoint -- --nocapture --test-threads=1
```

Require exactly one passed kernel test with 28 named create/submit successes, then
one passed oracle with 7,947×768 equality, `original_compiled=false`, the unchanged
corpus/output SHA256 above and both negative controls detected. Hardware mode
never compiles the known-broken frozen OLD kernel; it compares the entire captured
CPU-original result, including untouched words. Do not export/refresh the golden,
remove a corruption or weaken equality to make a native pass.

Then require R12=1, receipt=1, sleep=7 and checkpoint=6 passed on the same admitted
release route. Further native compiler failures identify remaining gates; this
single repair is not evidence that all other entrypoints compile. Any failure
keeps this candidate unqualified. No PR/merge or held-source action is included.

Care continuation still requires matching native/inspector binaries and full
Current/Adam/private-world/recurrent/sleep/RNG/pending-seed admission on a
disposable copy; preserve semantic_prior identity and all acquired learning.
Existing collection cutoff is 14:57 UTC, hard end 15:00 UTC on 2026-11-02. The
sole PC owner retains all admission, resource, capture and cleanup obligations.
