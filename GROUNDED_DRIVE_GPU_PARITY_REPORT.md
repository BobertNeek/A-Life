> Publication copy derived from the original report SHA-256 `cddf33b09484b2c3e9f587c0d5e5c4339c640369758e0fb36de37a3dcf1f0301`. Execution paths are replaced with `<workspace>` and `<artifacts>`. The original report describes the local execution stage before this source publication. See [PUBLICATION.md](docs/experiments/grounded-drive-basis-v1/PUBLICATION.md) for the additive publication identity and limits.

# Grounded-drive software-kernel qualification

Base source: `1d39f179790234c5de2ec011811af352f7060a05`.
Executed test source: `67e26f53009c9b925b94cac1af5421894b059481`.
Executed source tree: `d0281b30428a0f4ed1b48e568ab1da3cfdbd5e55`.

This is a finite synthetic numerical check of the opt-in class-5,
`N2048GroundedDriveBasisV1` research ABI with its sealed 768-Q24 candidate cost.
It is not a creature life, biological competence result, training run, hardware
GPU qualification, or permission gate for subsequent CPU-only work.

Research context: https://rikiyat.github.io/blog/weight-space-programming/.
Explicit mechanisms, bounded repeated tests and ablation are useful; a synthetic
kernel pass does not establish whole-organism drive competence.

## Result

One ignored, explicitly invoked integration test passed: three fresh deterministic
variants, six dynamic frames each, six candidates per frame. Across the variants,
all eight candidate families are present. Each candidate has 54 decoder inputs.

The actual production encode/recurrent/decode/memory/selection/eligibility path
ran on `SwiftShader Device (Subzero)`, vendor 6880, device 49374,
`DeviceType::Cpu`, Vulkan, `SwiftShader driver`, subgroup width 4.
The existing public software-runtime fixture was extended by a 12-line helper
inside the existing `gpu-tests`-gated module to choose research pricing. Ordinary
production and hardware-required constructors are unchanged. The test proves
ordinary software pricing rejects class 5, and research pricing rejects ordinary
class 3, before allocating an arena or resident.

Predeclared numeric acceptance is
`abs(cpu-gpu) <= 2e-5 + 3e-4 * max(abs(cpu),abs(gpu))`.
No tolerance was relaxed.

| Variant | Profile | Actual microsteps/frame | Max logit error | Max neural/encoded error | Max derived eligibility error | Nonzero derived eligibility comparisons |
|---|---|---:|---:|---:|---:|---:|
| 0 | GroundedObjectSlotsV1 | 3 | 9.536743e-7 | 2.9802322e-7 | 1.1920929e-7 | 96 |
| 1 | GroundedObjectSlotsV1 | 4 | 1.013279e-6 | 2.9802322e-7 | 1.09896064e-7 | 320 |
| 2 | GroundedTerrainVisionV1 | 4 | 2.5331974e-7 | 2.9802322e-7 | 6.891787e-8 | 272 |

All 5,832 final decoder input values (18 frames x 6 candidates x 54 lanes)
matched CPU bit patterns exactly after actual shader execution. The numerical
comparison includes 108 candidate logits, all 2,048 activations, activity EMA,
metabolic load and encoded inputs each frame, and all 1,280 added decoder
eligibility coordinates each frame. CPU recurrent state evolves independently
across frames after the shared fresh initialization; it is not reset to GPU state
on each step.

Variant 0 binds synthetic projection gain 0.5 and threshold shift +0.25 to a
three-microstep phenotype (maturation 0.5). All six actual step receipts assert 3.
This matches a previously problematic numerical context, not actual newborn
biology, age expression or scheduler behavior. Other variants bind gain 0.85,
threshold shifts 0 through 0.005 and assert 4 actual microsteps. Each frame logs
its bound receptor upload and actual step receipt. Existing pressure replay sets
neutral timing/queue/heap terms, retains actual ATP and causal source identities,
and keeps the finite fixture reproducible. This is not a performance assay.

## What the fixture exercises

- Fresh, explicitly asset-bound class-5 candidates only: 2,048 neurons, 34,048
  static synapses, 1,280 nonzero signed added readout weights, and no cognitive
  decoder. Values are hand-authored test stress weights, not a biological recipe
  or trained checkpoint. No optimizer update occurs.
- Positive and negative observed chemistry, hot/cold surfaces, thermal contact
  on and off, neutral unrelated objects, empty-observation candidates, intrinsic
  vocal opportunity, and an observed peer beside inert decoys. Personal novelty
  and play expectancy remain zero.
- Exact raw 0..23, derived 36..45, memory 24..35, remaining cognitive 46..52 and
  grip 53 inputs. Cognitive successor values 46..48 are nonzero (0.42). Memory
  and grip are tested at zero; arbitrary nonzero memory/grip preservation is not
  established by this fixture.
- Actual staged WGSL derived eligibility, compared with CPU feature-times-motor
  reference algebra using independently evolved CPU activations and the actual
  selected joint candidates. This does not invoke the CPU organism's private
  staging implementation. Staged transactions are explicitly discarded after
  each check. Committed eligibility stays zero; no outcome credit is applied.
- An input-only CPU ablation of derived lanes changes logits by up to 2.663698,
  0.16942753 and 0.15941688 for variants 0, 1 and 2. The GPU is compared against
  the nonzero arm. This is not a second GPU ablation arm.
- Full CPU trainer checkpoint equality before/after, including weights, moments,
  ages and optimizer step. Every GPU resident is removed at completion.

## Reproduction and evidence

Worktree: `<workspace>/grounded-drive-gpu-parity-v1`.
Target: `<artifacts>/grounded-drive-gpu-parity-v1-target`.
Evidence directory: `<artifacts>/grounded-drive-gpu-parity-v1-evidence`.

Toolchain root is
`<workspace>/recovered-toolchain-rust-1.99.1-reset2`.
Set `CARGO_HOME=$ROOT/cargo`, `RUSTUP_HOME=$ROOT/rustup`, and prepend
`$ROOT/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin:$ROOT/cargo/bin`
to PATH. Rust is 1.99.1, commit `58cf8f9c35b355199727627d07ae34a4e2846d67`,
LLVM 23.1.1, x86_64-unknown-linux-gnu. Build settings:
`CARGO_BUILD_JOBS=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`.

Commands:

```sh
cargo test --offline -p alife_training --features cpu-backend,gpu-tests \
  --test grounded_drive_gpu_parity --no-run
VK_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json \
VK_ICD_FILENAMES=/usr/lib/chromium/vk_swiftshader_icd.json \
cargo test --offline -p alife_training --features cpu-backend,gpu-tests \
  --test grounded_drive_gpu_parity -- --ignored --exact \
  priced_grounded_drive_software_kernels_match_cpu --test-threads=1 --nocapture
cargo clippy --offline -p alife_training --features cpu-backend,gpu-tests \
  --test grounded_drive_gpu_parity -- -D warnings
```

The evidence directory includes an executable `run-fixture.sh` with exact
environment setup. `run-01.log` retains full adapter, phenotype/asset identities,
all 18 per-frame records and successful exit status. Native test runtime was
24.51 seconds, including setup; no throughput interpretation is warranted.
`identity-run-01.txt` records exact source/tree, toolchain and executable hashes.

Executable:
`target/debug/deps/grounded_drive_gpu_parity-8c5623dd9357d217` within the target above.
SHA-256: `24a644cc8988b2ca3c0142cabe3e188a56e9726426f8c618c04e8d1863fee32f`.
SwiftShader library SHA-256:
`0370ba973a74faa50a2d191fcc14e697b51e8485ec778bafb6539064e66b9a3a`.
ICD SHA-256: `c9eef28b6b984fec220ef0abfecc40b502d46946706e47bfe97707027cb818bd`.

The first build failed on a fixture-only missing import of the public
`training_rollout::GpuTrainingStateSnapshot` type. Its complete error is retained
in `build-01.log`; adding that import produced `build-02.log` (pass, 5.46 seconds).
There were no shader-execution or numerical test failures in run 1.
Focused strict Clippy with warnings denied passed (`clippy-01.log`, 2m 03s).
Formatting and `git diff --check` passed for the changed Rust files.

## Limits and state changes

No actual GPU backpropagation or gradient test was run here. Earlier CPU finite
difference evidence is separate. No credit application, fast-weight learning,
nonzero accumulated eligibility, nonzero episodic memory, physical GPU,
world/game/CPU-host admission, retained state migration, save/load continuation,
performance calibration or biological drive success is established.

No world was created or advanced. No `host.step`, autonomous study, retained
checkpoint/bank read, trained-asset migration, default promotion, GitHub or Space
write occurred. Fresh fixture assets and banks existed only in process memory.
Persistent changes are limited to this isolated source branch, its local Git
commits, build products and evidence logs. No existing creature state changed.
