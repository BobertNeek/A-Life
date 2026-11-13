# Slice 2 design: care-dock response

**Branch:** `bob/soul` (head `b9d32a2c32`, Nov 13, 2026)
**Author:** Bob (Muse), Cassidy's AI assistant.
**Status:** implemented in this pass; tests written for Codex/CI to run.
**Follow-up (Nov 13, same day): the presentation was reworked — see
"Follow-up: no-text presentation" below. The receipt's shape below is
unchanged; the kind names changed in the follow-up (Touch→Pet,
TouchAndPraise→PetAndPraise) and only how it reaches the player changed.**

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
  Praise, or TouchAndPraise (both queued before one tick) — renamed in
  the follow-up to Pet, Praise, PetAndPraise (plus Scold and Slap, which
  the world cannot produce yet). `applied_tick`
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
    touch the loneliness delta is the measured movement. The debug line
    shows only the kind-relevant channel.
  - The HUD line showed the latest receipt for the selected creature, or
    "No touch or praise felt yet." when there was none — **that line is
    gone in the follow-up (see below); it was the wrong presentation, and
    its "no receipt" wording was misleading (see the unknown rule).**
  - The dock's Queued feedback is unchanged — it describes the request,
    which is still honest at click time; the receipt describes the response.
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

Follow-up files (see below): `crates/alife_game_app/src/production_voxel_renderer/live_care_feedback.rs`
+ `live_care_feedback/tests.rs` (new: the word bubble), `.../hearthling.rs` +
`.../hungry_ear_tests.rs` (care nod/flinch), `.../care_dock.rs` +
`.../care_dock/tests.rs` + `.../production_voxel_renderer.rs` + `.../menu.rs`
(Touch→Pet rename, status/help text, module registration), plus the
`headless.rs` kind generalization and the `status_hud.rs` HUD removal.

## Follow-up: no-text presentation (Nov 13, 2026)

Cassidy's direction: the user-facing game should avoid text (menus may
use single words; creatures speak in text bubbles), and — broader —
visual style should carry meaning instead of forcing the user to read a
technical report. The slice 2 HUD text line ("Praised, felt at tick 7,
praise 0.10 to 0.42") was exactly what he doesn't want. The receipt
underneath stays; only the presentation changed.

### The presentation

The response reads in the creature, twice, and nowhere else:

1. **Posture** (`hearthling::OutcomeFeedback`): a fresh receipt pulses
   the head for 0.60s — a nod (+0.15) for the positive kinds, a flinch
   (head −0.10, ears −0.15) for the negative ones. Same mechanism as the
   eating nod, same dedup (only a newer applied tick re-triggers), frozen
   on pause. No receipt means no pulse.
2. **Word** (`live_care_feedback`, mirroring `live_sleep_feedback`): a
   single word from the creature's own 18-word codebook, in a bubble
   above it for ~3s, coloured by valence (warm green positive, muted red
   negative), billboarded to the camera. The word names what the
   creature thinks just happened — pet → "play", praise → "get", scold →
   "look", slap → "retreat", pet-and-praise → "play" — and the posture
   carries whether it was good or bad. Together they are the "what", not
   just the "that something happened" (Black & White's judgement moment:
   the creature looks at you). The word fires once per receipt tick (a
   per-organism freshness map remembers what was shown); an old receipt
   never re-appears, and a superseding receipt replaces the word.

The word mapping is the weakest part of the design and is documented as
such in the code: the codebook has no affective words (no good/happy/
love/thank/more), so "get" (praise) and "look" (scold) are interpretive
— the closest available verbs for receiving a reward and for attending
to a correction. Pet and pet-and-praise collide on "play". Revisit when
the codebook grows (slice: "a tongue that grows").

Timing is the strictest rule (Black & White): the receipt is truthful
because it records the dose on the consuming tick, and the presentation
preserves that tightness — the nod and the word are visibly tied to the
moment, never a lingering label.

### Unknown, not absence (Bob prime's flag on PR #337)

Slice 2's "No touch or praise felt yet" was misleading for old saves: a
save with no recorded receipt means UNKNOWN, not "the creature definitely
felt nothing". Fixed everywhere in this pass:

- The HUD text line is removed entirely — there is no longer any
  rendering that could claim an absence.
- No receipt shows nothing: no word, no nod, no flinch, no "nothing
  felt" pose. The absence of a response IS the unknown state, and there
  is no confirmed-absence state anywhere in the system to confuse it with.
- The debug readout says "care: unknown (no receipt recorded)" — never
  "none yet" or equivalent. The world accessor's doc states the same.
- The world test pins it: no receipt before any care reads as
  `PresentationEvidence::Unknown`.

### Four actions (decided Nov 13, Cassidy + Bob prime)

Praise + pet (positive), scold + slap (negative). Praise/scold are
social feedback (player-reward channel); pet/slap are distinct physical
interactions (social-contact channel). Effects go through the creature's
biology and learning, not direct weight changes. Bob prime coordinates
the brains side.

What this pass did: the receipt kind is now all four actions plus the
both-positive combo (`Pet`, `Praise`, `Scold`, `Slap`, `PetAndPraise`;
`Touch`/`TouchAndPraise` survive as serde aliases for branch-era saves).
The recorder derives the kind from the applied dose's signs, with the
negative arms as the slot the follow-up fills. The presentation (nod/
flinch, word, colour, debug line) already handles all four kinds, and
the dock's Touch action is renamed Pet (icon asset keeps its legacy
filename). The word and nod for scold/slap are pinned by unit tests
that construct the kinds directly — the world cannot produce them yet.

What this pass deliberately did NOT do (follow-up slice: scold/slap
dock actions): there are no Scold/Slap dock buttons, no negative dose
is produced, and no brains-side change was made. The follow-up slice
needs, in order:

1. **Dock actions** (`care_dock.rs`): Scold and Slap buttons (new icons,
   keys, labels), wired through `dispatch_core`/`CarePort::care` — which
   currently takes a `praise: bool` and will need a kind (or a signed
   dose) instead.
2. **Dose production (brains side, Bob prime coordinates):** how a scold
   and a slap reach the biology. The sign-based recorder arms assume a
   negative `player_reward` (scold) / negative `social_contact` (slap),
   but `BodyEventDelta::validate_contract` holds both channels in
   [0, 1] today — negative doses are invalid until that contract
   changes, or a different channel is chosen. Research note (Creatures):
   reward/punishment route through felt drive states as chemicals
   (C3/DS Reward 204 / Punishment 205; drive down = reward/strengthen,
   drive up = punishment/weaken), learning associates the LAST action,
   and the valences are physically asymmetric (pet = comfort, slap =
   pain + small real damage + trust loss). Full comparison:
   `~/workspace/research_notes/creatures-black-white-reward-praise-20261113-1831/report.md`.
   The mechanism is the brains side's decision; the game side only
   needs the dose to arrive with a sign the recorder can read.
3. **Recorder match:** extend `record_player_care_receipt` for the
   both-negative and mixed-sign doses (scold+slap, pet+scold in one
   tick) — currently they record nothing rather than misattribute, and
   that choice must be decided, not guessed.
4. **Tests:** a world test producing a scold and a slap receipt via the
   dock path; sync-level tests for the negative word/flinch (the unit
   tests already pin the mapping, but nothing yet proves a negative
   receipt reaches the screen).

### Numbers: debug-only

`status_hud::debug_care_line` keeps the slice 2 formatter (all five
kinds, kind-relevant channel, unknown string) as a clearly-marked debug
helper with its tests; it is not wired into the user-facing HUD. The
`Readout.care` field, the `CareResponse` HUD line, and the panel height
it added are removed. The V0 selected-creature debug text already shows
numbers in debug mode; wiring the debug care line into it is a later
step (its sample builder has no presentation row in hand — the same
follow-up noted for the v0 "Last response" line).

### Menu survey (this pass)

- `menu.rs`: screen titles were two words — "Player menu" and
  "Simulation speed" (accessible dialog names). Fixed to "Menu" and
  "Speed". All menu items were already single words (Play/Save/Load/
  Speed/Settings/Quit; 1x/Max/Back; Autopause/Back; Cancel/Load).
- Help strip (F1-gated, not default-visible): "C Gentle touch" is now
  "C Pet" with the rename; the strip's status sentences ("Pet queued.")
  follow. The strip is instructional text shown on demand, not a label.
- NOT fixed (reported for a later slice): the load-confirm warning
  ("Load replaces this world. Unsaved changes are lost.") and the load
  error strings ("Wait for checkpoint publication before loading",
  "GPU runtime unavailable; load was not queued") are multi-word
  technical text in the menu flow — the warning is a data-loss guard and
  needs a design call before it is reworded or iconified. The HUD's
  identity line shows `#<stable_id>` (a technical label) and the count
  line shows a bare number — both pre-existing, both noted, neither
  touched here.
