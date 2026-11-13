# World-issued grounded source availability V1

This opt-in observation API distinguishes an unobserved feature from a measured zero without changing current sensory values. It is a building block for later partial-observability recall, not a new memory system or demonstrated navigation improvement.

`HeadlessWorld::perception_frame_with_availability_v1` returns the ordinary current frame and a non-serializable `GroundedAvailabilityFrameV1`. Only the World assembler constructs its private fields: organism, actual tick, final frame digest, and each grounded source's tracked identity, availability mask and actual sensor gains. `matches_frame` checks the binding. A caller-supplied arbitrary snapshot cannot issue a receipt.

Supported profiles are `GroundedTerrainVisionV1` and `GroundedTerrainObjectPortsV2`, with homeostasis at the actual World tick. Unsupported profiles and wrong times are rejected before tracking. Legacy APIs and checkpoint formats are unchanged; no automatic game/collector caller is enabled.

## Groups and calibration

`known_groups()` is a bitmask: RGB 1, material 2, shape 4, contact chemistry 8, contact temperature 16. Nonzero Vision gain admits appearance. Actual physical contact plus nonzero Chemical gain admits source-specific chemistry; temperature follows physical contact. This matches the existing extractor: chemistry/temperature are calibrated before the separate Touch gain, so zero tactile contact must not erase a measurement available to another channel.

Raw contact comes from the very same World snapshot as calibrated slots. Measured neutral contact and unknown distant features may share zero numeric values but differ in availability. Hidden sources receive no receipt. Anonymous smell, feedback and current values are not relabeled as remembered object properties.

## Current-stimulus compatibility

One ordinary tracking/calibration pass is used. The receipt binds the final frame after all pending Praise/Scold/Pet/Slap cues. Reading does not consume their doses, advance physics or biology, or apply outcome credit. Ordinary identity-tracking bookkeeping may change as in a normal frame read; this is not a claim that all World state is immutable.

Tests cover legacy-frame parity, contact versus distant, measured neutral versus unknown, hidden sources, frame/tick/profile rejection, rejection without extra tracking, pending four-cue parity and capability calibration. Initial test failures used the generic builder's larger food radius; the final fixture explicitly uses radius 0.25. No production contact threshold changed.

## Admission boundary

No serialized memory record/query format, memory bank, learner, collector, organism ATP debit, cold-host admission, learned behavior, mental map or GPU execution is added here. This is an externally requested observation primitive. Future organism-memory integration must authenticate/persist metadata transactionally and account for work before claiming improved recall. Hosted workflows are not represented as executed validation.
