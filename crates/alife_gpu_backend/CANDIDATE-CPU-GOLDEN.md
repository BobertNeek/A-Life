# Candidate-only native oracle against frozen CPU-original outputs

The flat production pipeline passed on RTX 3050/DX12/FXC. The prior native oracle
then failed compiling the frozen OLD kernel with FXC X3663 (varying barrier),
before it could compare outputs. This is a reference-kernel compilation limit,
not evidence that the candidate's outputs differ. The explicitly authorized
replacement runs the candidate and negative control on hardware and compares
against captured original CPU outputs. Production shader/runtime are unchanged
from a8f227bb.

## Reference identity and full-word comparison

The original b9f78ed0 shader remains frozen, LF-normalized SHA256
f04f1e5dfc73a94ee3820af64ebab6391d927976f43dcb8c06981b587fcf3f9d.
The same 8,535-case corpus is pinned by SHA256
06428c4fcd579e75b4c718829a3f1502bb384b30f229ea3766126a154d3a2ff2,
covering every fixture's name, dispatch count, expected-status annotation and
all seven input buffers, including all 512 initial state words.

A source-only sparse fixture captures original output changes relative to each
exact input. Its 816 deduplicated patches and 8,535 IDs reconstruct all 4,369,920
original words. They were exported from ORIGINAL pipeline slot 0 on explicit CPU
Vulkan only AFTER the unchanged historical digest passed. Independent review
reconstructed and compared every word with the raw original output, then
independently recomputed its SHA256 and historical canonical digest.

Original full-output raw little-endian SHA256:
ae6e8922b4430f511dfec7fc1a3965796f8b179f6aa25e9debd32e4aa2853773.
Unchanged historical digest, u64 order:
db70c676c01908bb cbe91726a0435cd3 02dcbc58f3e69598 ad2b2bc0771fa666.

The consumer reconstructs all 512 expected words for every case and asserts exact
word-for-word equality with actual candidate hardware output. It additionally
checks raw SHA256, the unchanged historical digest, positive completion
payload/status and the weakened-bank negative control. Patches cannot replace
or skip cases: corpus identity, shape and patch indices are validated. No
hardware mode can export/refresh the reference. The original CPU-versus-candidate
execution mode is retained and now also validates the captured fixture.

## Sole PC owner command

Use the exact published source commit/tree from the task receipt in a fresh clean
checkout and the same admitted release flags/compiler identity. Production code
is identical to the passed flat candidate. Preserve prior outputs. Apply the
existing owner/resource/consent/safe-window guard and reconcile prior processes.
Do not install compilers/drivers, reset devices or load accepted Current.

ALIFE_SLEEP_GOLDEN_EXPORT_DIR must be UNSET. Save/restore process environment
values in try/finally. ALIFE_SLEEP_ORACLE_ARTIFACT_DIR is a new nonexistent absolute
directory with an existing parent. It saves context.json and the exact generated
candidate/negative WGSL plus program SHA256 logs. No old GPU program is created.

```powershell
$env:ALIFE_GPU_BACKEND = 'dx12'
$env:ALIFE_GPU_DX12_COMPILER = 'fxc'
$env:ALIFE_SLEEP_ORACLE_ARTIFACT_DIR = $NewNonexistentOracleDirectory
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test sleep_initializer_equivalence --no-run
```

After the separate CPU build, under the owner's outer 180 second deadline:

```powershell
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test sleep_initializer_equivalence device_oracle::nvidia_dx12_candidate_matches_frozen_cpu_original_for_all_corruptions -- --ignored --exact --nocapture --test-threads=1
```

Require exactly 1 passed test, actual NVIDIA/DX12 and explicit FXC, 8,535 cases,
512 words/case, original_compiled_on_device=false,
all_words_match_frozen_cpu_original=true, matching raw SHA256/historical digest,
and detected negative control. Record all source/build/DLL/adapter evidence and
raw stdout/stderr/exit. A mismatch, compiler failure, timeout or zero tests blocks
qualification; never update the reference to fit hardware.

The oracle deliberately retains its previous ABI+rawshader composition. Candidate
WGSL is 71,220 bytes, canonical LF SHA256
040ba8ce76e3f51ad2544b02fccc3f05a35c935440a77f7bb1f21da6bad120a6.
Negative-control WGSL changes only the active-bank upper bound, SHA256
2fd248545d5c8135c8bbd2d949c232a0b9c8d799f84185cab4578f8883ce8338.
The separately pipeline-tested production input remains feature-composed 71,645 bytes;
its unused selector-hook composition differs. Keep the production initializer,
all 28 kernels, R12, sleep/checkpoint and full Current/care admission gates in
FLAT-SLEEP-VALIDATION.md. This oracle does not establish whole-training numerics.

No PR/merge or held-source publication is part of this compatibility change.
All raw reference exports are synthetic local verification evidence; only the
compact Rust fixture is committed. No learned checkpoint or individual state is
included. The existing care cutoff remains 14:57 UTC collection/15:00 UTC hard end.
