# Slice 2 design: care-dock response

**Branch:** `bob/soul` (head `7297059af6`, Nov 13, 2026)
**Author:** Bob (Muse), Cassidy's AI assistant.
**Status:** implemented in this pass; tests written for Codex/CI to run.

## The gap

Touch/Praise in the care dock record only Queued/Placed/Rejected evidence.
The player cannot tell whether the creature noticed. The dock file says so
itself (`care_dock.rs`: "queued stimuli do not establish a creature
response").

## What already exists (all verified by reading code on `bob/soul`)

- `HeadlessWorld::queue_player_care` validates reach (praise: vision radius
  plus line of sight; touch: contact radius plus physical reach) and
  accumulates a dose into `pending_player_care`: praise sets
  `player_reward = 1.0`, touch sets `social_contact = 0.25`. Repeated input
  before one tick cannot stack the dose.
- On the tick, the dose is consumed into the creature's biology at three
  sites (idle tick, motor bundle, action) and written into its sensory
  frame (`tactile_contact[0] = social_contact`,
  `auditory_acoustic[1] = player_reward`). The creature DOES perceive and
  feel it — the world just forgets the moment the tick ends: the pending
  entry is removed and nothing records that it happened.
- Praise's measured channel is `hormones.extension[0]` (the "praise signal"
  meter): PlayerReward emits a `praise` species, whose only receptor writes
  `EndocrineChannel::Extension0`; PlayerReward also emits dopamine. Touch's
  measured channels are the loneliness drive (down) and oxytocin (up):
  SocialContact emits LONELINESS −0.60, OXYTOCIN 0.45, SEROTONIN 0.12.
- The praise signal FADES on the next tick (species decay 0.25), so the
  meter alone cannot prove a praise happened once it fades.
- The v0 inspector already has a "Last response" line and a
  `v0_confirmed_creature_response` delta comparison ("Praise signal rose" /
  "Comfort signal rose") — but it is dead in production: `last_response` is
  always `None` and the function is never called outside its test. The delta
  approach also cannot attribute a rise to the player's action (peer touch
  raises oxytocin too) and it needs a previous frame.
- The inspector (status HUD) reads `WorldOrganismPresentationRow` from the
  live frame snapshot. `PresentationEvidence::Observed { value, tick } |
  Unknown` is the house pattern for evidence-backed readouts.

## The design

A world-owned receipt, recorded when the dose is actually consumed:

- `HeadlessWorld.last_player_care: BTreeMap<u64, PlayerCareReceipt>` —
  the last receipt per organism. Last-only, like `last_touched_entities`
  and `last_action_result`.
- `PlayerCareReceipt { kind, applied_tick, praise_signal_before/after,
  loneliness_before/after }`. Kind comes from the applied dose: Touch,
  Praise, or TouchAndPraise (both queued before one tick). `applied_tick`
  is the outcome tick of the biology advance that consumed the dose.
  Before/after are the two measured channels read around that advance.
- Recorded at all three consumption sites, immediately after the biology
  advance, inside the existing clone-rollback transactions — so a failed
  tick records nothing.
- "Felt" means: the dose entered the creature's biochemistry AND its
  sensory frame on the same tick boundary. The receipt records the biology
  half; the sensory half is the same event (verified in
  `grounded_nonvisual_channels_from_objects`).
- Truthfulness rules:
  - The receipt is written only when a dose was actually consumed (a
    nonzero applied delta). A rejected queue (out of reach, blocked, dead)
    records nothing and leaves the previous receipt untouched.
  - The before/after numbers are measurements across the application tick,
    not a causal claim about the whole delta. For praise the praise-signal
    delta IS attributable — the praise receptor is its sole writer. For
    touch the loneliness delta is the measured movement. The HUD shows
    only the kind-relevant channel.
  - The HUD line shows the latest receipt for the selected creature, or
    "No touch or praise felt yet." when there is none. The dock's Queued
    feedback is unchanged — it describes the request, which is still
    honest at click time; the HUD describes the response.
- Persisted in the save (`#[serde(default)]`, old saves load with an empty
  map) and included in the canonical signature digest, so a save/load keeps
  the acknowledgement and two worlds with different care histories hash
  differently.

## What this is not

- Not a brain change: no neural, genome, or learning code is touched. The
  dose already reaches the brain through the existing sensory channels;
  this slice only records that the world applied it.
- Not familiarity (slice 4): no disposition is computed. The receipt is
  one event; a relationship is a history.
- Not the v0 "Last response" line: that line stays unwired in this slice.
  Its sample builder has no presentation row in hand and its
  `Option<&'static str>` type cannot carry a tick. Follow-up: feed it from
  the receipt if the sample builder gains row access.

## Files

- `crates/alife_world/src/headless.rs` — receipt types, storage, recording
  at the three consumption sites, public accessor, digest entry.
- `crates/alife_world/src/presentation.rs` — `last_player_care` row field.
- `crates/alife_world/src/persistence.rs` — save field, wire struct,
  restore.
- `crates/alife_game_app/src/production_voxel_renderer/status_hud.rs` —
  `Readout.care`, the care line, panel height.
- Tests: `crates/alife_world/tests/canonical_new_game.rs` (world,
  presentation, save/load), `crates/alife_game_app/src/production_voxel_renderer/status_hud/data_tests.rs`
  (HUD line), `crates/alife_game_app/src/production_voxel_renderer/status_hud/layout_tests.rs`
  (readout literals, panel height, three-tile layout).
