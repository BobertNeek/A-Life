# Converged publication barriers: complete production pass

Actual RTX 3050/DX12/FXC d139e44c gets past the repaired pending finalizer and
fails the next discard finalizer with X3663 after 16 successful submissions.
A focused audit of every packaged production entrypoint found 25 entries / 28
compiled variants. Seven entries contain nine barriers, all workgroup_size(1).
The existing initializer and pending finalizer are already converged. Exactly
five further entries have early returns before barriers: discard finalization,
fast-plasticity initialization/finalization, sleep staging and sleep commit.
The four later kernels are source-demonstrated instances of the same pattern;
they are not claimed to have produced a recorded native failure yet.

All five use the previously reviewed mechanical transformation. Their complete
pre-first-barrier bodies are copied into barrier-free helpers; only rejection
returns become zero readiness results. All rejection-side receipt writes remain.
Success returns explicit readiness and captured publication addresses. The
entrypoint executes every barrier unconditionally and guards only the original
success-publication stores. Discard preserves barrier/pending-valid=0/barrier/
receipt-success=1 order. Other entries preserve staging/barrier/status order.
No arithmetic, predicate, load/snapshot order, atomic ordering, ABI, dispatch,
durable format or biology is removed or changed. Invalid invocations gain only
empty barriers within a one-invocation workgroup. No inter-workgroup barrier
semantics are invented. Every surrounding shader byte remains unchanged.

Frozen full OLD sources are pinned from d139e44c in tests/fixtures, along with
CPU-original sparse output fixtures. Each of five independent corpora compares
all 512 mutable-state and 256 frame-payload words. Header, brain, extension, learning, identity, outcome/request, pending/parameter
and receipt regions receive bit and boundary corruptions, plus carries/wraps, discard staging-clear
checks/overlaps and plasticity rejection contexts. Each corpus uses one row;
these tests do not claim new multi-row or whole-training numerical coverage.
The pre-existing pending corpus retains its disjoint/mixed-row checks.

Explicit CPU Vulkan executes OLD/candidate/two negative variants with independent
resets. Normal strict reread compares OLD and candidate to the pinned output,
not a regenerated golden. Guard negatives reach the selected helper/validator.
Discard's publication negative reverses both stores around the same barriers and
is detected by two overlap cases; other publication negatives corrupt success
status. Independent review verified exact staging-prefix preservation and
reconstructed all 35,341,824 output words from raw CPU-original streams.

| Entry | Cases | Guard / publication negative cases | Full CPU-original output SHA256 |
|---|---:|---:|---|
| `finalize_discard_pending_eligibility` | 9597 | 1 / 2 | `5bf43349a249623f473cc5047dab1312961a21e4feae8b94c3e90cd783f6ba09` |
| `initialize_fast_plasticity` | 9927 | 3 / 3362 | `cbe81b4ca0151bc385383f37dacb89e42d401e5ed4aaf0e422ec0f4a19ec4903` |
| `finalize_fast_plasticity` | 9932 | 40 / 9780 | `2133520f53aa75398037ac4353cca5af0af992e533874a36bc706248bd5a522e` |
| `finalize_sleep_staging` | 8282 | 21 / 4511 | `be30a6116fded35f17c7ac0a17abd324942ee968deb09eb2ca3ad85e8568a3d3` |
| `finalize_sleep_commit` | 8280 | 47 / 4597 | `56b60157a72000adc492f866074a599cc850f9b8dbce937b22a063afcf175d93` |

The total is 46,018 cases × 768 words. Every corpus and output SHA is pinned in
its checked-in fixture and checked before/after native execution. All-five strict
reread and the whole-set control-flow audit pass, as do all 28 software production
kernel create/submits and 12 CPU WGSL/layout checks. The audit rejects nested or
helper barriers and any entry return before its final barrier. Implicit Naga
returns after publication are harmless and allowed. CPU evidence does not qualify
RTX execution. Full original floating-point paths remain unchanged; any native
FTZ/NaN/numerical mismatch remains a qualification failure, never a golden refresh.

## Sole PC owner execution contract

Materialize the exact published commit/tree receipt in a clean isolated worktree.
Retain the existing owner, consent, process/resource/lease, compiler DLL identity,
release flags, capture and safe-window guards. No reset/install/driver change,
Current access or private learned-state mutation is part of these synthetic tests.
Keep ALIFE_SLEEP_GOLDEN_EXPORT_DIR, ALIFE_PENDING_GOLDEN_EXPORT_DIR and
ALIFE_BARRIER_GOLDEN_EXPORT_DIR unset. Use fresh artifact directories, save raw
stdout/stderr and exit status, and preserve/restore environment in try/finally.
Build with --no-run separately and retain source/executable/compiler hashes and
adapter/backend/driver receipts. Apply the existing outer 180-second deadline per
GPU process (300 seconds for complete sleep/checkpoint targets), one process at
a time. Reconcile an unsuccessful process before another authorized run.

The pending oracle is independently runnable even when a later production kernel
fails: it compiles only its own target and negative controls, not the discard or
other finalizers. The five-kernel oracle likewise never compiles OLD on FXC.
A failed kernel gate blocks production qualification/care, not independent bounded
oracle diagnostics after process reconciliation. These are separate probes, not
an unconditional shell chain that proceeds to training after a failure.

```powershell
$env:ALIFE_GPU_BACKEND='dx12'
$env:ALIFE_GPU_DX12_COMPILER='fxc'
$features='gpu-tests,training-rollout,pipeline-diagnostics'

$env:ALIFE_PENDING_ORACLE_ARTIFACT_DIR=$NewNonexistentPendingDirectory
cargo test --release --locked -p alife_gpu_backend --features $features --test pending_eligibility_equivalence device_oracle::nvidia_dx12_fxc_matches_frozen_cpu_original_all_corruptions -- --ignored --exact --nocapture --test-threads=1
Remove-Item Env:ALIFE_PENDING_ORACLE_ARTIFACT_DIR

cargo test --release --locked -p alife_gpu_backend --features $features --lib closed_loop_pipeline::kernel_diagnostics::nvidia_dx12_each_production_kernel_compiles_and_dispatches -- --ignored --exact --nocapture --test-threads=1

$env:ALIFE_BARRIER_ORACLE_ARTIFACT_DIR=$NewNonexistentFiveOracleDirectory
cargo test --release --locked -p alife_gpu_backend --features $features --test publication_barrier_equivalence device_oracle::nvidia_dx12_fxc_all_five_match_frozen_cpu_original -- --ignored --exact --nocapture --test-threads=1
Remove-Item Env:ALIFE_BARRIER_ORACLE_ARTIFACT_DIR
```

Require pending=1 passed, 7,947×768 equality and its unchanged d139 output SHA;
kernel=1 passed with all 28 named create/submits; all-five=1 passed with five
completion records totaling 46,018×768 equality, original_compiled=false, all
five pinned hashes and both negative controls per entry. Exactly zero executed
tests is a failure. Artifact directories retain the actual candidate/negative
WGSL inputs; compiler/runtime HLSL hashes are separate identities.

After those gates pass on the admitted FXC release route:

```powershell
cargo test --release --locked -p alife_gpu_backend --features $features --test closed_loop_fast_plasticity r12_native_success_promotes_exactly_one_transaction -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features $features --test closed_loop_runtime hardware::hardware_receipt_is_one_bounded_required_backend_contract -- --exact --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features $features --test closed_loop_sleep -- --nocapture --test-threads=1
cargo test --release --locked -p alife_gpu_backend --features $features --test closed_loop_checkpoint -- --nocapture --test-threads=1
```

Require R12=1, receipt=1, sleep=7 and checkpoint=6 passed. Actual same-individual
care still requires matching native/inspector binaries and disposable-copy full
Current/Adam/private-world/recurrent/sleep/RNG/pending-seed admission, preserving
semantic_prior identity and acquired learning. Existing collection cutoff remains
14:57 UTC, hard end 15:00 UTC on 2026-11-02, including capture/cleanup reserve.
No PR, merge, held-source publication or unqualified care continuation is included.
