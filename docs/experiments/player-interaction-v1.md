# Four player experiences, V1

`HeadlessWorld::queue_player_interaction_v1(organism, source, kind)` accepts
`PlayerInteractionV1::{Praise, Scold, Pet, Slap}`. The old bool care API remains
an exact Praise/Pet wrapper. The new API returns Queued or NoReceivedDose;
a zero-Hearing Scold is not described as queued. The input is an experience, not a command selecting
the creature's action, and does not write a reward verdict or a synaptic weight.

## Explicit design defaults

- Praise preserves existing PlayerReward 1.0 and acoustic lane1.
- Pet preserves SocialContact .25; it is not nutrition.
- Scold supplies an aversive vocal social cue, not injury. Its received dose is
  Hearing gain clamped to [0,1], with no delivery for an absent hearing port.
  That same once-received dose enters acoustic lane2 and the appended
  SocialDisapprovalV1 source. An explicit fresh-genotype builder adds Fear .12
  and Cortisol .15 emitters with neonatal expression. No loaded genotype is
  rewritten; inherited missing/altered emitters and receptors remain meaningful.
- Slap delivers Damage .02 through existing physical injury, nociception and
  stress biology, plus tactile exposure. Injury resistance and inherited
  receptors matter. Repeated physical punishment can harm the creature.

These numbers are game design choices, not measurements from Creatures or
Black & White. Those games inspire four distinct inputs and the importance of
feedback timing, not exact chemistry or guaranteed understanding.

Social cues use the existing praise range/line-of-sight validation; physical
contact uses existing contact reach. Invalid/dead/missing targets reject before
mutation. Repeated queued input of the same kind does not stack before a
biological boundary. Different kinds coexist; praise/scold do not cancel.
A held UI button must issue discrete inputs, not claim immunity across future
biological boundaries. There is no persistent absence-as-punishment stimulus.

## Timing, attribution, receipts and compatibility

Existing idle, registered-action and motor-transaction boundaries consume the
pending player-only dose once. Failed transactions preserve pending input.
Save/load preserves pending doses. New event fields default to zero and omit
zero on serialization; the new source enum is appended (old ordinals fixed).
No neural/checkpoint/optimizer fields, learned weights, or moment histories are
migrated or reset. Canonical fresh-game genotype construction explicitly opts
in the new social source; prior saved creatures retain their actual graph.

The queued source has no authenticated earlier-action identity. Ordinary
biology and current-interval outcome credit may affect decayed prior eligibility
and the current action's eligibility. This is NOT proven A-specific credit.
Idle can consume feedback without a sealed motor-action credit packet. No UI
should claim that a particular action was learned or that the organism felt or
understood the event merely because it was queued.

Muse owns presentation and applied-care receipts separately. A receipt's kind
set must derive from the consumed **player-only** dose, never merged action
Damage (a collision is not a slap). Mixed inputs need a set/mask, not a single
exclusive label. Report event time/dose and measured physiology separately;
no record means unknown, not that no interaction ever occurred. Biology
changes across the interval are not proof that the player alone caused every
change. No full game PR merge is part of this feature.

CPU core/world tests are not GPU neural qualification or evidence of learned
behavior. Hosted workflows did not run due the repository's allocation issue.

### Existing positive-input sensory limitation

The legacy Praise/Pet explicit acoustic/tactile projection is terrain-profile
specific. Their biochemical delivery is shared across profiles. This patch does
not silently revise those legacy profile semantics. The new Scold and Slap cues
are projected across all grounded profiles, using Hearing and Touch calibration
respectively; Slap can still injure a touch-insensitive body. Neither an applied
biochemical dose nor the presence of a sensory channel proves felt experience
or understanding. Privileged debug profiles are not qualified as player-input
perception by these tests.

The fresh canonical recipe adds two emitter evaluations to the existing honest
biochemical work receipt, even with no Scold. Legacy loaded graphs retain their
original work/state behavior. Therefore no-input bit identity is claimed for
unchanged legacy graphs, not for every work counter of a newly opted-in founder.
