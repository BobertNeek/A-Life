# Synthetic GPU actor rejection and value rollback

The actual public PPO controller passed one focused rollback qualification on
software Vulkan. Both ordinary execution and the disabled frozen-gradient
experiment rejected the final actor Adam proposal after submitting the encoder
containing the positive value update. Persistent actor and value state remained
exact. The experiment remains disabled; this establishes neither physical GPU
qualification nor a performance result.

The tested source is `76ad6c3ce6d13a569cf595436820903a38bf6860`, based on
`b1fd11290c99ec0ca6aa457a466cca046831885e`. The only trainer change is a
`gpu-tests`-gated read-only diagnostic for the same gradient-norm/validation words
already read by actor submission. PPO source and both production shaders are
byte-identical to the base; removing the diagnostic restores exact base trainer
source. No failure hook, normal admission change, or controller/math/configuration
change was added.

## Fault and controls

The existing procedural N2048 fixture has 32,768 synapses and one trainable
ActionCandidate/Approach decoder at index 25,728. One real accepted warm update
establishes actor/value step 1, policy 5, and nonzero moments. Two immutable replay
windows have 3 and 4 loss rows. The qualification uses one epoch, effective batch
8, value coefficient 0.5, and the existing actual controller.

The fault changes only that decoder's first moment to finite `f32::MAX`, retaining
its actual age 1 and the warmed weights, second moments, configuration, and mask.
Both `validate_for` and `restore_checkpoint` accept it. This is deliberate
contract-valid synthetic fault injection, not a claim that ordinary learning
produced this history. Decoder sign policy is unconstrained. Adam bias correction
overflows the prospective update; the shader's whole-update validation rejects
it before any actor coordinate is written. Actor moments do not enter forward,
PPO objective, raw gradient, or value calculations.

Each execution path also runs a matched positive control with the ordinary
warmed first moment. Both controls accept one update, advancing actor/value
steps to `[2, 2]` and policy to 6. All 2,049 coordinates change in each persistent
value parameter, first-moment, and second-moment plane.

Both fault paths return `MalformedReadback` after the same loader order
`[0, 1, 0, 1]`. Actual GPU readback shows finite accumulated gradient norm
`0.12669114768505096`, bit-identical to the positive controls, actor validation
flag 0 (controls: 1), and value preflight 1. Actor weights, moments, integer ages,
configuration and host step remain exact; value parameters/moments, value step,
and policy remain exact at `[1, 1]` and 5. The 32,767 frozen actor coordinates
remain unchanged.

The ordinary and cached captures match all 278,101 u32 words for the accepted
controls, and another 278,101 words for the faults. Four captures retain 1,112,404
words total. These cover logical replay outputs/state, actor checkpoint arrays
and ages, durable value state/counters, all five value planes, and PPO objective
adjoints/metrics/values/preflight. Private raw actor-gradient/backward-tape
buffers are not captured.

The two value-gradient scratch planes are compared between post-error paths,
not against pre-cohort contents. Matched controls observe active accepted value
updates. For faults, source command ordering establishes value apply before
actor preflight in the submitted encoder, followed by awaited value rollback;
transient value writes are not sampled.

## Reproduction and evidence

Build the exact source in a fresh target directory:

```sh
CARGO_TARGET_DIR=/path/to/unique/target cargo test -p alife_training \
  --features gpu-tests --test software_ppo_controller --no-run --message-format=json
```

Select and hash the `software_ppo_controller` executable from the Cargo
`compiler-artifact` event, then invoke that same executable with an available
software Vulkan ICD and:

```sh
ALIFE_DISPATCH_REQUIRE_HARDWARE=0 \
ALIFE_SOFTWARE_PPO_CAPTURE_DIR=/path/to/synthetic-output \
  /path/to/bound-test-executable \
  rollback_qualification::actor_gpu_rejection_restores_value_update_for_original_and_cache \
  --exact --ignored --nocapture --test-threads=1
```

One focused test passed in 4.80 seconds. Targeted integration-test Clippy with
`--features gpu-tests -- -D warnings` passed, as did formatting/diff checks.
This was an unoptimized correctness build with rustc 1.99.1, running actual
`DeviceType::Cpu`, Vulkan, llvmpipe LLVM 19.1.7 / Mesa 25.0.7. A unique target was
seeded only with third-party artifacts; all workspace `alife_*` artifacts were
excluded, and the build log confirms current-worktree compilation.

The bound executable SHA256 is
`f1c5e307d816479032e4f569c2e427d96e204bec081755d63b5fc25b46bf6bc5`.
[The receipt](20261101-frozen-gradient-rollback.json) preserves source/tree/file
hashes, Cargo artifact/profile/features, six library hashes, exact invocation,
actual adapter identity, pass log, strict lint log, and per-category word counts
and little-endian u32 SHA256 digests. Independent source/evidence review found
no blockers and recomputed the complete ordinary/cache comparisons and hashes.

The 7,933,463-byte raw capture is retained outside Git at
`/workspace/software-ppo-frozen-rollback/rollback-semantic-words.json`; its SHA256
is `cf49307625762cca6b249d9031d5fd3c3ab9c92bb81a12428742f33f1d28779a`.
The source fixture regenerates it through the capture environment variable.
No real checkpoint, Windows/PC work, driver/device-loss injection, or performance
assay was used. PR225's head and all production defaults remain unchanged.
