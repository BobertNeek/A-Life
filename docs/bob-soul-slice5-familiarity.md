# Slice 5 — Familiarity record

A first-class, observable disposition toward the player, built from
interaction history. Vision pillar 2: "Creatures that remember you — an
observable disposition (familiarity/trust/wariness) from real interaction
history, shown in the inspector, never invented."

## The record

`PlayerFamiliarity` (`crates/alife_world/src/headless.rs`) is world-side
history, per organism, last-only like the care receipt:

```rust
pub struct PlayerFamiliarity {
    pub value: f32,                  // in [-1, 1]: -1 wary, 0 stranger, +1 trusting
    pub last_interaction_tick: Tick, // the tick the last applied dose was felt
    pub interactions: u32,           // applied doses felt so far
}
```

Stored in `HeadlessWorld.last_familiarity: BTreeMap<u64, PlayerFamiliarity>`,
persisted in `WorldSaveState.last_familiarity` (serde default — old saves
load an empty map, which reads as unknown), included in the canonical
digest under `"player-familiarity-v1"`, and projected into the presentation
row as `PresentationEvidence<PlayerFamiliarity>`.

This is the player's standing with one creature. It is **not** the memory
bank's `object_familiarity`, which is a creature's familiarity with a
tracked object from retained episodes — a different axis, a different
owner. No name or field is shared between them.

## What feeds it

The same source as the care receipt: the **consumed player-only dose**,
never the button pressed and never the merged body event. The update runs
at the same three consumption sites, immediately after
`record_player_care_receipt`, with the same dose and tick. The kind set is
now derived once, by `PlayerCareKindSet::from_dose` (extracted from the
recorder, which calls it too), so the receipt and the record can never
disagree about what was applied.

| Applied kind | Delta |
| --- | --- |
| Pet | +0.06 |
| Praise | +0.04 |
| Scold | −0.10 |
| Slap | −0.15 |

A mixed dose sums (pet + scold in one tick nets −0.04). The result clamps
to [−1, 1]. A zero dose records nothing and changes nothing — including a
zero-hearing scold, which queues `NoReceivedDose` and leaves no record.

The deltas are tuning, debug-visible only. The asymmetry is deliberate:
negative kinds cost roughly twice what positive kinds build, so one slap
costs more trust than one tickle builds, and a creature that has only ever
been scolded reads wary, not neutral. Pet outranks praise because physical
contact carries more social weight than a social cue. Kind **presence**,
not magnitude, moves familiarity: a barely-heard scold counts the same as
a clear one. Only the scold's magnitude varies (hearing gain), and the
receipt already carries that subtlety; the record keeps the simpler rule.
If that simplification ever reads wrong in play, it is a tuning change,
not a redesign.

The record is fed from the applied dose, **not** from the measured
channels (praise signal, loneliness, fear). The channels are
genotype-dependent — fear, for example, only moves on genotypes that opted
into the Scold source — so a creature that felt a scold but shows no fear
would otherwise record no memory of it. The dose is the fact; the channels
are the response.

## Decay

Familiarity fades toward neutral when the game runs without interaction.
There is no per-tick write: the stored value is the value at the last
interaction, and the current value is derived lazily —

```
value_at(now) = value * 0.5^((now - last_interaction_tick) / 72_000)
```

— applied on update (the stored value is decayed to the interaction tick
before the new delta is added, so a long-ignored creature starts from
where its memory has faded to, not from its last high) and at snapshot
time (the row carries the decayed value). Half-life is 72,000 ticks: one
game-hour at 20 world ticks per second
(`PlayerFamiliarity::HALF_LIFE_TICKS`). Ticks only advance while the game
runs, so a creature does not forget the player while the game is off; it
fades across a session spent ignoring it. An hour of neglect halves the
bond; two hours quarter it. That is a tuning guess, like the deltas — the
doc and the tests name the number so it can be argued with.

## Unknown

No record is **unknown**, never hostile, never trusting, never a confirmed
absence. A fresh creature, a creature never yet cared for, and a save from
before familiarity existed all read exactly the same way. The distinction
that matters: a record decayed to ~0 reads **Stranger** (history netted
neutral), which is different from no record at all. The presentation
surfaces keep the two apart everywhere: the pose adds no offset for
unknown, the meter shows its "?" marker with a hidden track, and the
debug line says "unknown (no interaction history)".

## Bands

Interpretive thresholds for the debug line only — presentation vocabulary,
never a brain input, never a number at the user:

| Band | Range |
| --- | --- |
| Trusting | ≥ 0.5 |
| Familiar | 0.15 – 0.5 |
| Stranger | −0.15 – 0.15 |
| Uneasy | −0.5 – −0.15 |
| Wary | < −0.5 |

`PlayerFamiliarity::band()` is the single definition; the debug line is
its only consumer.

## What the player sees

Two channels, both wordless. Nothing numeric reaches the user; the exact
value lives in the debug readout only.

### 1. Resting posture (primary — always visible)

The creature carries its standing in its head and ears, continuously, in
`hearthling.rs` (`outcome_rotation`, after the sleeping early-return, so a
sleeping creature shows no familiarity pose):

- head pitch = value × 0.08, ear pitch = value × 0.08
- trusting (+1): head forward, ears up; wary (−1): head back, ears flat
- unknown: no offset — a stranger stands as it stands

The amplitude is about half the care pulse (nod +0.15, flinch −0.10/−0.15),
so the disposition reads as carriage, not a reaction. It composes with
the pulse: a fresh nod on a trusting creature reads larger, a flinch on a
wary one reads deeper, and both are real signals, so their sum is honest.
It also composes with the afraid expression (head −0.10, ears −0.35): a
wary creature that is also afraid shows both. The value is re-read from
the row every changed frame and stored on `HearthlingVisual` as
`familiarity: Option<f32>` — a standing value, never an event, never
latched. The diagnostic dump (`capture_feedback`) carries it.

### 2. Inspector meter (secondary — the selection panel)

A seventh `Quantity::Familiarity` in the status HUD, mapped
`(value + 1) / 2` (wary empty, stranger half, trusting full), unknown as
the "?" marker with a hidden track. It never warns — familiarity is a
relationship, not a need — and its fill is the same green as the other
non-warning meters. Icon: a heart, `assets/ui/status_hud/familiarity.png`
(96×96, the house off-white `#FAF5D6`, drawn to the same bar as the other
icons). The panel widens 370 → 430 (`PANEL_WIDTH`, one const used by the
spawn, the layout, and the height computation) so seven 52px cards plus
six 8px gaps fit in one row on desktop — 7×52 + 6×8 = 412 inner, exactly
the old six-card fit extended by one card. Widening rather than wrapping
keeps the selection panel fitting above the care dock at 640×360, which
the layout test pins. At 320px wide the panel still wraps to two rows,
exactly as before, and `panel_bottom(320, 240)` is still 236.0.

### 3. Greeting word — deferred

The word system (`live_care_feedback`) fires on a fresh receipt tick; a
greeting needs a different trigger (the creature noticing the player, or
approaching) and the codebook's affective words are already the weakest
link ("get" for praise, "look" for scold are interpretive). A greeting
word now would be invented meaning. Recorded as an open item, not
implemented.

### 4. Approach / avoidance movement — not implemented, Codex question

Steering a creature toward or away from the player is brain-side. The
game side cannot move a creature without touching the brain, and the
brains are Codex's side. If the brain wants a familiarity observation as
a steering input, that is a question for Bob prime / Codex, flagged here
and not implemented. The posture and the meter carry the disposition
meanwhile.

## The hard boundary

Familiarity is a world-side record derived from interaction history plus
presentation. It reads biology signals (the dose fields) and writes
nothing into learning: no weights, no genome, no brain input, no change
to how the brain learns. The band enum is presentation vocabulary, not a
brain input. If a design ever needs familiarity to influence learning,
that is a Codex decision, not a field in this record.

## Tests (for Codex/CI — none run here)

World (`crates/alife_world/tests/canonical_new_game.rs`):

- `player_familiarity_builds_on_pet_and_praise_erodes_on_scold_and_slap_and_survives_save` —
  unknown before care, pet +0.06 on the consumption tick (not the queue),
  praise stacking to +0.10, scold netting to ~0.0, slap to −0.15 with the
  Uneasy band, the snapshot carrying the decayed value stamped with the
  last interaction tick, and the record surviving a save/load round trip.
  The 1e-4 tolerance absorbs the ~1e-5-per-tick decay between the
  test's interactions.
- `player_familiarity_records_nothing_for_a_zero_hearing_scold` — the
  deaf-game mirror of the receipt test.
- `player_familiarity_decays_toward_neutral_with_a_one_hour_half_life` —
  pure math on `at_tick`: unchanged at the same tick, halved at one
  half-life, quartered at two, negative values decaying toward zero too,
  and a tick before the last interaction never moving the value.
- `player_familiarity_bands_split_trust_from_wariness_with_stranger_in_the_middle` —
  every boundary value.
- `player_familiarity_clamps_at_both_ends_of_the_scale` — 17 pets clamp
  to exactly 1.0, 14 slaps from there clamp to exactly −1.0.

Presentation (`hearthling.rs` tests):

- `familiarity_pose_is_a_quiet_resting_carriage_not_a_pulse` — unknown
  leaves the sampled pose untouched; +1.0 pitches head and ears exactly
  +0.08; −1.0 exactly −0.08; sleeping suspends the pose; a nod on a
  trusting creature reads larger than the nod alone. The eleven existing
  `outcome_rotation` call sites pass `None` for familiarity.

HUD (`status_hud/data_tests.rs`, `status_hud/layout_tests.rs`):

- The physiology test is now seven quantities: the row with no history
  reads `None` for familiarity, a record at 0.5 reads `Some(0.75)`, and
  familiarity never warns.
- `set_raw` gains a familiarity arm; the invalid/valid-value test uses
  familiarity's own domain ([−1, 1] raw, mapped to [0, 1]) instead of
  the 0..=1 lanes' domain.
- The rebind test asserts the six physiology lanes read while familiarity
  stays unknown in a world with no care history.
- The layout counts move 6 → 7 (fills, tracks, unknown markers, warnings)
  and every `Readout` literal gains a seventh value; the partial-unknown
  case leaves familiarity unknown, exercising its "?" marker.
- `debug_familiarity_line` pins the unknown string and three band cases.
- The embedded-icon test covers the new heart: 96×96, distinct hash,
  opaque and transparent pixels.

## Open items (not fixed here)

- The v0 "Last response" dead machinery (never called outside its unit
  test; its delta approach cannot attribute a rise to the player's
  action). Follow-up decision stands: feed it from the receipt or delete
  it.
- The load-confirm warning and load-error technical text in the menu
  flow (a data-loss guard needing a design call).
- The HUD identity `#<stable_id>` line and the bare creature-count line
  (pre-existing, untouched).
- The care-word size/height tuning guesses (font 18.0, height 2.10).
- The greeting word (deferred above) and approach/avoidance movement
  (Codex question above).
- The kind-presence simplification: a barely-heard scold moves
  familiarity as much as a clear one. If play shows that reading wrong,
  scale the scold delta by the hearing gain.
- The half-life (one game-hour) and the four deltas are tuning guesses,
  named in one place each so they can be argued with and changed.
