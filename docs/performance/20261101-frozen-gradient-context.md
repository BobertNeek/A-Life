# Nonempty dendrites and changing private replay context

One actual-controller software Vulkan test passed with nonempty dendrites and
changing structural/effective-weight context. Removing each context component
individually changed GPU logits and **96 trainable recurrent raw gradients**.
After real recurrent updates, normal and experimental cached PPO still matched
all captured words, checkpoints, receipts and loader order. This is synthetic
correctness evidence, with no performance or physical GPU claim.

The fixture retains a nonempty branch into the actual selected decoder source
neuron, with two active inputs and a negative threshold. The branch
descriptors are immutable. Frozen structural edges vary through counts
0/1/2 across ticks, with changing negative weights on the actual compiled ActionPlanning
lateral-inhibition route and its exact cadence. Only the selected decoder coordinate changes each tick in an otherwise-zero
32,768-entry offset vector. This does not qualify broad active offsets or
recurrent-offset coverage. The public replay validator accepts all
contexts; no trained parameter slots are added for frozen structural edges.

Each branch/structural/offset removal and the combined no-context ablation uses
the same fixed adjoints and warm actor boundary. Every ablation changes actual
GPU logits and finite trainable recurrent gradients. Separately, normal PPO on
an independently collected no-context trajectory yields different recurrent
optimizer moments; its old probabilities and values are collected under its
own context, avoiding stale-policy inputs.

The full eight-tick GPU trajectory supplies seven continuing/truncated loss rows
and a real nonzero final bootstrap. Global GAE is computed before splitting into
three/four loss rows, with replay shapes 3/burn0 and 6/burn2. For the suffix, actual
after-tick0 activations, activity EMA and metabolic state are copied, and the
nonempty branch descriptors retained. WGSL computes branch output from previous
activation plus those immutable descriptors; it owns no separate dynamic branch
register. **Every one of the six suffix rows**, including burn-in, matches the
full replay's logits, values and all three neural-state arrays bit for bit.

The every-third mask trains 10,923 of 32,768 synapses. A real warm update first
seeds nonzero actor and value-head history. A one-epoch probe changes a recurrent
weight and both moments at 171 coordinates before any later epoch. Exact warm
checkpoint rewinds then compare two accepted normal epochs against two cached
epochs; 171 recurrent coordinates advance, all trainable integer ages increase
by 2, and all 21,845 frozen coordinates preserve weight/moment/age bits. Value
parameters and both moment planes advance from step 1 to 3. Cache diagnostics
show only two raw windows reused; later evaluation is fresh.

Executed source is clean commit `b886a8ab0db00295af2dacd522dcf71bf5791512`, based on
`b1fd11290c99ec0ca6aa457a466cca046831885e`, on the separate branch
`codex/frozen-gradient-context-qualification`. Binary SHA256 is
`0a4f12a838011ca2941e9a70668df97dc5356bc6f62365637a66ed95d9638269`.
A unique Cargo target forced current local crate compilation. Root and an
independent helper reviewed the fixture before execution. The
[receipt](20261101-frozen-gradient-context.json) retains source/tree/binary
hashes, raw build/test logs and strict relevant Clippy output. A later test-only
iterator idiom adds an explicit six-value length assertion while retaining the
same six equality checks. The focused test was not repeated.

The explicit selector0 and actual AdapterInfo identify CPU llvmpipe/Vulkan under
an unoptimized correctness profile. No PC, personal checkpoint, performance
assay, production implementation, shader, default activation or PR225 head was
changed. Memory-cap fallback, forced actor-apply/value rollback failure, physical
GPU qualification and maximal dendritic topology remain unqualified.

```sh
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export ALIFE_DISPATCH_REQUIRE_HARDWARE=0
export CARGO_TARGET_DIR=/workspace/A-Life-frozen-gradient-context/target/context-tests
cargo test --locked -p alife_training --features gpu-tests --test software_ppo_controller context_qualification::nonempty_dendrites_and_changing_private_context_match_fresh_ppo -- --exact --ignored --test-threads=1 --nocapture
```
