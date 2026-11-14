# Applied player care and dock presentation

Selective integration of the finished care-receipt presentation and Scold/Slap
dock controls from Muse's `bob/soul` at `a884cb90831f809e159493fc43d4f8c2271243b4`.
This does not merge PR #337 or its unrelated vision/harness changes. The
receipt presentation introduced in slice 2 is a required dependency of slice 3.

## Authority and interpretation

- The dock routes Praise, Scold, Pet and Slap through the existing canonical
  `queue_player_interaction_v1` backend. Queued is not applied. NoReceivedDose
  creates no queued marker or queued status; errors remain rejected.
- The receipt records a set of kinds from the consumed player-only dose on the
  biology tick. PlayerReward means Praise, SocialContact means Pet,
  SocialDisapprovalV1 means Scold, and Damage means Slap. Hazard damage, peer
  contact and other merged body events cannot manufacture player attribution.
- Missing receipt evidence is unknown, not proof of no feeling. It produces no
  word or posture. Zero dose records nothing; existing receipt history remains.
- The exact set persists. Old single-kind strings, including Touch and
  TouchAndPraise, are accepted for branch-save compatibility. Empty maps are
  omitted, preserving older saves and no-care state fingerprints.
- A visible response chooses one interpretive display kind: Slap, then Scold,
  then Pet, then Praise. This is not a neural report or a measured feeling.
  Positive responses nod; negative responses flinch. The bounded word choices
  are play/get/look/retreat, not new creature utterances or learning signals.
- Debug-only positive-channel numbers are whole-tick snapshots, not causal
  changes attributed solely to the player. Negative-only receipts do not claim
  unrelated praise/loneliness channels measure the negative response.

## Controls and interruption

The dock order is Food, Pet, Toy, Praise, Scold, Slap, Speech. V and B select
Scold and Slap. Modal blocking keeps V separate from the lineage shortcut.
Only living selected creatures receive these actions. Word cues are limited
to eight active/fresh candidates, never eight creatures with historical
receipts. Older/equal receipt ticks cannot replay. Unknown or invalid rows
clear active care presentation without fabricating new receipt evidence.

## Integration boundary

Main's #338/#339 chemistry, queue, hearing and grounded-sensor behavior and
#340 source-availability changes are retained. No core, training, genome,
neural, checkpoint or learned-state changes are part of this integration.
The original Muse branch and assets remain untouched. See the PR for exact
local validation and any hosted-CI limitations.
