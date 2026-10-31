# Continuing GAE after actual recurrent updates

The additional wide-mask continuing test passed on software Vulkan. The first
accepted update changed a recurrent weight **and both optimizer moments at 171
coordinates**. After two accepted epochs, 171 recurrent coordinates also differed
from the warm boundary. Normal and experimental controllers still matched every
captured word and receipt, while cache diagnostics showed only two reused windows
and fresh evaluation in the later epoch. This is correctness evidence only.

The existing coherent eight-tick fixture, real nonzero bootstrap and global GAE
were retained. Its three/four loss-row windows use the actual restored causal
state and two burn-in rows. The wide mask trains 10,923 coordinates, including recurrent coordinates. A one-epoch normal-controller probe first proves
recurrent weight/moment changes; the test then restores the exact warm actor and
value checkpoints and compares normal two-epoch training against experimental
shared-window training. The probe does not supply updated weights to either side.
All trainable integer ages advance by two; all 21,845 frozen coordinates preserve
weight/moment/age bits. Value parameters and both moment planes each advance from
nonzero warm history, with actor/value optimizer steps 1→3.

The original
[three-test qualification](20261031-frozen-gradient-continuation.md) and its exact
`c3b47a78` receipt remain unchanged. Those three tests were not rerun. This fourth
case required no fixture-input changes and no production changes. The only
additional shared helper change exposes an existing test-only progress check.

Executed source is clean commit `1a01abda66582ab01e35249cffe091bd09310766`, based on the pushed three-pass
milestone `8dc80934`. Binary SHA256 is `7bb1fafc39adbb5fd57c4d180a7ff5a25a302fb496ed0e21dc4059805d661170`.
The [receipt](20261031-frozen-gradient-wide-continuation.json) retains source/tree/
binary hashes, the focused build and raw test log, and strict all-target Clippy
output. Root reviewed the source and confirmed that the compiled non-decoder kind
is Recurrent. The explicit selector0 and adapter logs identify CPU llvmpipe/Vulkan;
no physical GPU, PC job, real checkpoint, timing assay or default activation was
used. Memory-cap fallback and forced actor-apply/value rollback remain unqualified.

```sh
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export ALIFE_DISPATCH_REQUIRE_HARDWARE=0
export CARGO_TARGET_DIR=/workspace/A-Life-frozen-gradient-continuation/target/qualification-tests
cargo test --locked -p alife_training --features gpu-tests --test software_ppo_controller continuing_qualification::wide_continuing_gae_keeps_later_epochs_fresh_after_non_decoder_updates -- --exact --ignored --test-threads=1 --nocapture
```
