# How creature memory currently works — 2026-10-01

Source: consolidated `2bad5e02b6bbf125a10822d3c7effd7debc96d8f`. The leaf-timing
patch changes no memory behavior described here.

A creature learns in two connected ways. Its GPU neural weights change through
plasticity, while its RAM memory bank stores compact experiences that can be
recalled before another action. Host metadata, upload staging and readbacks also
use RAM; saves, checkpoints and sleep journals create durable filesystem copies.
The [storage-path note](20261001-live-leaf-timing.md#ram-gpu-buffers-and-filesystem)
covers conditional cache/log writes and persistence triggers.

After a selected action has a sealed measured outcome, the ordinary live path
offers it to the creature's episodic bank. There is no minimum-importance
admission threshold. Invalid or duplicate/out-of-order experiences are rejected.
Each stored record contains a 96-value situation/action/target query, the tracked
object and action family, timestamps, observation count and confidence, plus
outcome summaries: hunger/fear/pain/curiosity/brain-energy changes, novelty,
contact, surprise, valence, success, danger and energy change.
These are [actual stored fields and their outcome mapping](../../crates/alife_core/src/memory/candidate_recall.rs#L5),
with [admission and sequence checks](../../crates/alife_core/src/memory.rs#L973).

The ordinary live bank holds **256 records per creature**. Very similar
experiences for the same identity merge into running averages. If the bank is
full, the next distinct valid experience replaces its least salient record;
ties choose the oldest and then the lowest record ID. Importance reflects
valence, pain, disappointment, prediction error and novelty.
[Live capacity](../../crates/alife_game_app/src/gpu_live_runtime.rs#L2429),
[merge and eviction](../../crates/alife_core/src/memory.rs#L1010).

Before an awake creature chooses, the app asks what memory predicts for each
candidate action. It does this once for initial attention evidence and again
after attention routes the final candidates. Object-target and action-family
channels search related indexed buckets, shortlist important/recent records,
and perform at most **64 similarity evaluations per channel per candidate**.
Matching uses fixed, weighted cosine similarity. The best **four** matches
scoring at least **0.72** contribute similarity-weighted outcome averages;
confidence also decreases with poorer similarity. A channel with no matches
returns neutral values with zero confidence.
[Retrieval](../../crates/alife_core/src/memory/candidate_recall.rs#L139),
[similarity and aggregation](../../crates/alife_core/src/memory/candidate_recall.rs#L322),
[shortlist ranking](../../crates/alife_core/src/memory/candidate_index.rs#L386).

Those recalled expectations influence attention and prediction, then enter GPU
candidate inputs. Learned GPU decoder weights turn the confidence-weighted
memory values into candidate-logit changes. Neural arbitration selects the
action; the CPU memory bank supplies evidence rather than choosing it.
The resulting real outcome can update both episodic records and neural weights.
[Production GPU memory decoder](../../crates/alife_gpu_backend/shaders/closed_loop_memory_context.wgsl#L104).

Sleep separates CPU record compaction from GPU replay and weight consolidation.
The ordinary CPU compaction request retains the existing bank capacity and
folds near-duplicates; it does not routinely lower the 256-record limit.
[Sleep compaction request](../../crates/alife_game_app/src/gpu_live_runtime.rs#L7475).

The present episodic implementation has concrete limits. It keeps fixed
numerical summaries, uses a hand-designed retrieval metric, and does not
automatically share object-bound recall across different tracked-object IDs.
Remembering alone does not strengthen or age a record. There is no time-based
record fading in this path; forgetting primarily comes from merging/compression
and capacity eviction. Neural-weight generalization and other cognitive systems
are separate from these specific episodic lookup rules.
