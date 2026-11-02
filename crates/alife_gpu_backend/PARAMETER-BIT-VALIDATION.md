# Serialized sleep-parameter bit validation

The flat FXC oracle rejected Parameters[1]=0x00000001: completion status was 0x200
instead of 0x100 and the preparation payload was unwritten. This word encodes the
smallest positive f32 subnormal, so the original CPU predicate staging_rate>0 is
true. The result is consistent with zero-flushing somewhere in the float path,
not yet proof of its exact operation. Same-flat DXC failed native PSO creation;
no additional DXC run is part of this packet.

Direct3D documents subnormal flushing during float operations and exceptions for
raw data movement. WGSL permits flushing in specified float operations and some
bit-reinterpretation intermediates. Consequently bitcast<f32>(raw) followed by a
bitcast back is not the basis for validating serialized bits. Sources:
[Direct3D float rules](https://learn.microsoft.com/en-us/windows/win32/direct3d11/floating-point-rules),
[WGSL floating-point rules](https://www.w3.org/TR/WGSL/#floating-point-evaluation).

## Narrow change and exact predicates

Initializer parameter snapshots now load raw u32 words 1/2/3. The shared sleep
validator snapshots the same eight immutable words as integers, after the same
bounds check. Four helpers implement the previous IEEE comparisons directly on
those encodings:

- Finite means the exponent is not 255.
- Greater than zero means a nonzero positive encoding no greater than +infinity;
  the separate earlier finite predicate still rejects infinity.
- Greater than or equal to zero also admits negative zero, but no negative
  nonzero value or NaN.
- Less than or equal to the fixed positive bound uses monotonic positive float
  encodings, admits negative numbers/negative zero, and explicitly excludes NaNs.

The fixed upper-bound encodings are 1.0=0x3f800000 and 8.0=0x41000000. Positive
subnormals remain positive; negative subnormals remain negative. Signed zeros,
NaNs and infinities retain the original predicate truth and validation order.
No input is clamped, normalized, rewritten or silently accepted. Other guards,
error/completion writes, atomics, barriers, bank/generation rules and format are
unchanged. The float loader and subsequent learning arithmetic are unchanged.
This repair does not emulate CPU floating-point arithmetic throughout training.

The unchanged 8,535 × 512 word original-output oracle, pinned historical digest,
raw-output SHA256 and negative control pass on explicit CPU Vulkan. Independent
review checked 204,365 encodings against IEEE truth and verified snapshots/bounds.
No source blockers found. GPU execution remains unqualified; do not promote the
candidate from CPU results or from a diagnostic-only probe pass.

## One focused FXC observation

Use the exact published source/tree, admitted release flags and compiler DLL,
fresh clean worktree/output, and the sole owner's existing process/resource/
consent/safe-window guard. Do not install/reset anything or access Current.
Build separately with --no-run; use an outer 180 second deadline per GPU process.
Retain raw logs, exit status and source/build/compiler/adapter identity.

```powershell
$env:ALIFE_GPU_BACKEND = 'dx12'
$env:ALIFE_GPU_DX12_COMPILER = 'fxc'
$env:ALIFE_PARAMETER_BITS_ARTIFACT_DIR = $NewNonexistentParameterDirectory
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --lib closed_loop_pipeline::parameter_bit_probe::nvidia_dx12_fxc_parameter_float_and_bit_observations -- --ignored --exact --nocapture --test-threads=1
```

It reads 26 representative encodings from immutable-plan storage: normal/subnormal
values of both signs, +/-0, boundaries around 1/8, finite maxima, +/-infinity and
quiet/signaling NaNs. It records old float and new bit predicates, CPU IEEE truth,
raw/roundtrip bits, NOT and atomicMax controls. Bit predicates and integer/atomic
controls MUST match. Native legacy-float disagreements are observations, not
qualification passes for the full oracle. For raw 00000001, a legacy >0 result 0
with integer >0 result 1 and intact integer/atomic controls supports the float-path
hypothesis. Record actual evidence rather than infer a general compiler defect.

## Mandatory full-state and production gates

Unset the component artifact variable after the probe, preserve/restore all
process environment values in try/finally, and set a NEW oracle output directory.
ALIFE_SLEEP_GOLDEN_EXPORT_DIR must remain unset. Keep strict DX12/FXC.

```powershell
$env:ALIFE_SLEEP_ORACLE_ARTIFACT_DIR = $NewNonexistentOracleDirectory
cargo test --release --locked -p alife_gpu_backend --features gpu-tests,training-rollout,pipeline-diagnostics --test sleep_initializer_equivalence device_oracle::nvidia_dx12_candidate_matches_frozen_cpu_original_for_all_corruptions -- --ignored --exact --nocapture --test-threads=1
```

Require exactly 1 passed test, 8,535 cases × 512 words, original_compiled_on_device=false,
all_words_match_frozen_cpu_original=true, the unchanged historical digest/rawSHA
and detected negative control. No reference/corpus/harness changes are part of
this repair. Candidate source/hash is recorded from this branch; historical
float-predicate source hashes in earlier packets are not this candidate.

Then retain all 28 production kernel compile/submit checks, production R12/receipt,
all 7 sleep tests and 6 checkpoint tests on the same release FXC route. Shared
validation changed, so these gates remain necessary even after initializer
success. Full Current/Adam/private-world/RNG/seed admission on a disposable copy,
matching native/inspector binaries, semantic_prior identity and acquired learning
remain required before authorized care continuation. Keep the established
14:57 UTC collection cutoff/15:00 UTC hard end and all ownership/capture/cleanup
obligations. No PR, merge or held-source action is authorized by these probes.
