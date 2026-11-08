# Continuing GAE and supervised fallback qualification

Three focused actual-controller tests passed on software Vulkan. This follow-up
qualifies continuing GAE and real speech/auxiliary fallback inputs for the
**disabled experimental** raw-gradient cache; it makes no performance claim.
Earlier receipts retain their original source and coverage limits.

The continuing test builds one coherent eight-tick GPU replay. Seven rows form
one continuing trajectory ending at an explicit truncation; the eighth observed
row supplies a real nonzero bootstrap value. Full-trajectory GAE is computed
before splitting into three/four loss rows, matching current orchestration.
The second replay window starts from actual post-tick0 activation, activity-EMA
and metabolic state, retains the empty dendrite context, and burns two rows.
Its logits and values match the corresponding original GPU rows bit for bit.
Artificially truncating at the window boundary or zeroing the final bootstrap
changes the targets, so the fixture exercises cross-window credit.

Normal and experimental controllers then accept two epochs from the same warm
actor/value history. Actor weights/moments/ages and each value parameter/moment
plane advance. Owned/shared checkpoint restores produce identical receipts and
all captured persistent, recurrent and objective words. Only two raw windows
are reused, and the later epoch evaluates afresh.

Two other tests use 10,923 trainable coordinates and put valid supervision only
on the second window, after one raw vector was cached. Speech supplies an actual
positive two-step token and a silence target; trainable speech-payload optimizer
moments differ from an otherwise identical no-label control. Auxiliary targets
select legal Approach candidate0 while PPO selected Idle candidate2; imitation
metrics are valid with positive loss, and optimizer moments differ from the
no-auxiliary control. Both cases accept two actor/value updates with nonzero
warm history, preserve frozen bits and integer ages, and match every captured
normal-controller word and receipt. Diagnostics show late discard: eligiblefalse,
one raw window, zero reuse, and 278,544 requested buffer bytes.

The executed source is clean local commit
`c3b47a789a8f95d81bc097146298db2f9c88003d` (see the exact identity in the
[receipt](20261031-frozen-gradient-continuation.json)), based on
`4ee3f6ca66a638e24004086e189253387063538e`. The branch is
`codex/frozen-gradient-continuation-tests`. Only test module hooks and two new
qualification modules changed; no production implementation or shader changed.
Root independently reviewed both modules before execution. Tests explicitly set
`ALIFE_DISPATCH_REQUIRE_HARDWARE=0`; actual logs identify CPU llvmpipe/Vulkan.
No physical GPU, PC job or real checkpoint was used.

All three tests passed under the unoptimized test profile. Their runtime and
whole-process RSS are process context, not throughput or cache-memory evidence.
Strict relevant all-target Clippy passed after one equivalent test-only chunks
iterator idiom change. The receipt retains the exact executed source/binary
hashes, build output and all raw logs.
Memory-cap fallback, forced actor-apply/value rollback failure and physical GPU
qualification remain untested. Production activation remains held.

```sh
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export ALIFE_DISPATCH_REQUIRE_HARDWARE=0
export CARGO_TARGET_DIR=/workspace/A-Life-frozen-gradient-continuation/target/qualification-tests
cargo test --locked -p alife_training --features gpu-tests --test software_ppo_controller _qualification:: -- --ignored --test-threads=1 --nocapture
```
