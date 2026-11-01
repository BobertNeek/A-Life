# Whole actor update rejection with a broad trainable mask

One focused synthetic software-Vulkan test passed the many-coordinate atomicity
gate. One invalid decoder Adam proposal prevented writes to every actor
coordinate, including other recurrent coordinates that demonstrably update in
the matched accepted control. Ordinary and disabled-cache execution matched
exactly. There is no physical GPU, performance, or default-enablement claim.

This supplements the [one-coordinate rollback case](20261101-frozen-gradient-rollback.md)
without changing its source or evidence. The tested source is
`ac3a949347e2ff8d7240cb304fbe00f5ab544341`, based on the published narrow-case
checkpoint `02e879cfdf04519a96277a4a4335d1b8a52dd4d3`. Only a separate ignored
integration-test module and its wiring were added. Trainer/getter, PPO,
production shaders, and all normal defaults remain unchanged.

The procedural N2048 fixture has 32,768 synapses, an every-third mask with 10,923
trainable coordinates, and 21,845 frozen coordinates. Two windows provide 3 and
4 loss rows, with all four microsteps active. One actual accepted warm update
establishes age-1 history. The qualification uses one epoch, effective batch 8,
and value coefficient 0.5.

Each matched ordinary/cache positive control accepts an update. The selected
decoder weight changes, and 171 other recurrent coordinates each change their
weight, first moment, and second moment together. All 10,923 trainable integer
ages advance exactly once; all frozen weights/moments/ages remain bit-exact.
Each persistent value parameter/moment plane changes at all 2,049 coordinates.
Actor/value steps advance from `[1, 1]` to `[2, 2]`, and policy advances from 5
to 6. The decoder's moment differences are not separately asserted; the
same-coordinate weight/moment witnesses are the recurrent coordinates.

The fault changes only the unconstrained decoder's first moment to finite
`f32::MAX`. `validate_for` and `restore_checkpoint` admit it. This is a deliberate
synthetic contract-valid fault, not naturally learned history. Both paths return
`MalformedReadback` with loader order `[0, 1, 0, 1]`. Actual GPU readback reports
finite norm `0.12799133360385895` (`0x3e031029`), bit-identical to the controls,
final actor validation 0 (controls: 1), and value preflight 1.

Without restoring the actor after failure, the fixture compares every one of
its 32,768 weights, first moments, second moments and integer ages against the
fault checkpoint: all remain exact, including the valid trainable coordinates.
Persistent value parameters/moments and actor/value host steps/policy also
remain exact at `[1, 1]` and policy 5. All five post-error value planes and all
other captured semantic words match between ordinary and cached paths. Value
scratch planes are not compared against pre-cohort contents. As in the narrow
case, transient value writes are inferred from source command ordering plus
matched accepted controls, not sampled during execution.

The ordinary/cache positive maps match 278,101 u32 words, and the fault maps
match another 278,101. Four captures retain 1,112,404 words. Capture scope and
exclusions are the same as the narrow case, including no private raw actor
gradient/backward-tape buffer capture.

The single focused run passed in 4.94 seconds with actual `DeviceType::Cpu`,
Vulkan, llvmpipe LLVM 19.1.7 / Mesa 25.0.7, using rustc 1.99.1 and an unoptimized
test profile. The current-worktree integration test was recompiled in its
uniquely owned target; the existing library bytes were unchanged. Targeted
strict integration-test Clippy, formatting, and diff checks passed. No rerun,
real checkpoint, PC work, driver/device-loss injection, or performance assay
was used. PR225 was not changed.

Use the narrow report's fresh-target build and Cargo-artifact binding procedure,
then invoke that bound executable with `ALIFE_DISPATCH_REQUIRE_HARDWARE=0`, a
synthetic `ALIFE_SOFTWARE_PPO_CAPTURE_DIR`, and:

```sh
rollback_wide_qualification::one_invalid_decoder_proposal_prevents_all_wide_mask_actor_writes \
  --exact --ignored --nocapture --test-threads=1
```

The executable SHA256 is
`68c54fae2761f871ef10c7a02ac08e5668eda17550756976a54f4740fe4afee4`.
[The receipt](20261101-frozen-gradient-rollback-wide.json) preserves exact source,
Cargo artifact, library/binary hashes, invocation/logs, actual adapter identity,
and every captured category's word count and little-endian u32 SHA256.
The 7,986,667-byte raw capture remains outside Git at
`/workspace/software-ppo-frozen-rollback-wide/rollback-wide-semantic-words.json`,
SHA256 `0c97f1b2980a876e055a4b6948cae815723bb835f9b88940ee1c85575f62c31b`.
