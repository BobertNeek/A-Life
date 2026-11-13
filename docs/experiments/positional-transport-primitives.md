# Detached positional transport primitives

This extends the default-off `experimental-decision-context` CPU feature with a
four-state, six-parameter numerical recurrence. It does not enable a body
collector, motor readout, GPU path, game default, full recurrent brain replay or
host save format. The separate experimental positional action head is excluded.

## Representation and ordinary inputs

State is `[relative world X / 32, relative world Z / 32, occupancy, confidence]`.
Fresh weights are `[.25, -.25, .25, .25, 1/32, 1/32]`. The metric calibration
and geometric accuracy apply to that fresh recipe; independently learned weights
remain permitted and do not guarantee metric or equivariant accuracy. Native yaw zero faces +X;
positive yaw rotates toward +Z. A captured vector is formed from sensed range
(normalized by 8), left/forward bearing, and sine/cosine of source gaze. A genuine
interval's measured own world displacement is subtracted once. Confidence ages
separately; fading confidence does not shrink the position toward the creature.

**Own interval displacement must be measured. Legacy action velocity is not a
reliable interval displacement, particularly during sleep:** a stale nonzero
velocity can persist when the body did not move. These detached functions do not
collect or authenticate those measurements. They also do not infer movement of
the remembered object. A moving target requires a new observation.

The sensed range is 3D while the bearing is planar. The numerical geometry was
qualified in flat-Y scenes, not as general 3D localization or multi-object memory.
Raw descriptor matching, source provenance, delayed-delivery interval ledgers and
current sensory encoding remain outside this layer. No privileged object ID or
target position is supplied to this recurrence.

## Cadence, identity and adjoints

`boundary=true` authorizes one numerical transition. A capture replaces the
stored position and a measured displacement may then be applied. Additional
microsteps use boundary=false: exact state identity, not repeated transport or
confidence decay. An empty sequence performs no work. Boundary=false also has
exact four-state adjoint identity, including signed zero, and zero input/weight
adjoints even for extreme finite unused fields. Input validity is still checked.

There is no reference-tick or motion buffer in this detached library. The caller
must not advance a source/reference tick, consume buffered measurements, deliver
a source, or invent a transition when no neural update is admitted. On resuming,
it must bind each real outstanding measured interval exactly once. A reference
frame held during sleep is not evidence that the last action's velocity should
be integrated. Actual host admission and interval provenance require separate
qualification, not simply a call to `forward2013`.

Coordinates use hard bounds [-1,1], with derivative 0 at and beyond either bound.
`backward2013` supplies analytic six-parameter, four-initial-state, seven raw
source/motion-input and two confidence-input adjoints through the supplied
transition sequence. Capture and boundary are provenance events, not fitted
continuous controls. This is BPTT through this small recurrence only, not through
the original 2048-neuron brain. A tape is bound to the exact forward weights;
changing model weights before backward is rejected. All returned adjoints must
be finite. Optimizer moments and per-parameter integer ages are independent;
failed updates do not partially commit. No existing optimizer history is reset.

## Work and serialization

Each transition reports one dispatch and four state reads. An active transition
also reports 17 input/parameter reads, 43 scalar arithmetic operations and four
neuron updates. A zero-pulse transition does not acquire those active costs.
These are explicit logical kernel units, not measured FLOPs or an ATP debit.
Source quaternion preparation, interval-decay preparation, validation, hashing,
allocation and collector bookkeeping are outside these counts; a real host must
charge its actual extra work and apply its combined debit policy before admission.

The existing experimental model/state/input serialization identities are retained
for this standalone layer. No full experimental host/checkpoint schema is added
to main or migrated. Ordinary defaults and acquired lives remain unchanged.

## Provenance, corrections and validation

Source: frozen `relative-location-native-2013/transport-qualified-v2`, handoff
SHA-256 `435e4e0748c86dcba34358cce1afe88a7b1cb75f3c8b40d007454ce4d5b5c7be`.
The test-only frozen reference preserves its ordinary numerical arithmetic.
The extracted integration adds an explicit zero-pulse backward branch: evaluating
unused products first could overflow even with finite inputs, breaking identity.
Inactive capture-feature adjoints likewise skip unused products on an active
no-capture transition, while motion adjoints remain active. Saturated zero-adjoint channels skip their
unused range sums before overflow as well, preserving ordinary nonzero arithmetic
order. It also checks every returned input adjoint and rejects forward/backward model
mismatch. Frozen research files are not modified.

Tests compare ordinary states bit-for-bit and adjoints against the frozen
reference across both mirrors and cadences 1/3/4/7, and separately check finite
differences, saturation, capture/replacement, confidence, independent Adam/cold
state, invalid input, overflow and zero-pulse extremes. Passing these tests is
not evidence of better physical navigation, autonomous forgetting, motor-head
qualification, or GPU support.

```
cargo test -p alife_training --features experimental-decision-context --lib decision_context
```
