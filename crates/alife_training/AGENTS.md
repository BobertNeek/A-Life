# alife_training

Own offline foundation training, curricula, optimization, and evaluation. GPU and
opt-in CPU backends execute the actual compiled neural and learning semantics;
keep execution domains explicit in results. Training is separate from live game
execution.

Preserve retained weights, moments, frozen coordinates, private state, and exact
integer optimizer ages across native continuation. Bind continuation to the actual
source checkpoint; do not reset learned state to make a handoff pass. Use the
existing `FoundationWeightAsset` codec and preserve frozen weights bit-for-bit.

Keep optimizer state, training targets, and auxiliary heads out of gameplay saves.
Evaluate through ordinary perception, world legality, biology, and real outcomes;
synthetic scored frames do not demonstrate creature learning or gameplay.
