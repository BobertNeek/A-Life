# Gameplay system review — 2026-09-26

Review order agreed with the user:
1. Senses, neural inputs, actions, consequences.
2. Food, digestion, energy, hunger, fatigue, sleep.
3. Learning, chemical credit, memory, prediction, consolidation.
4. Development, reproduction, newborns, ageing, death.
5. Player care, communication, visible feedback.
6. Saves, checkpoints, archives, lineage.
7. Ecology, tooling, performance.

## First group: sensory and action boundary repairs

Requirements: AOA-SENSE-001/002, AOA-MOTOR-001 through 007,
AOA-AUTH-001 through 005, AOA-MEM-001 through 003, AOA-PERSIST-003.
This is a source/contract review, not proof of learned navigation or visible play.

For GroundedTerrainVisionV1, retain the existing 220-degree view, independent
head swivel, opaque occlusion, transparent sight passage, and coarse depth fan.
Add short forward steps and body turns to existing motor channels. Their
candidates are unscored, targetless, and geometrically parameterized. Movement
uses the same world collision and outcome path as other walking. Hold-gaze lets
the brain retain its chosen head offset while walking or turning its body.
Candidate features distinguish head and body actions; at most three visible
objects receive five choices each, leaving the frame within the 32-choice ABI.

Anonymous smell uses two local samples of four physical odor bands. Strength
depends on distance and expressed chemical sensitivity. It supplies no source
identity or precise target. This is a bounded radial approximation; walls do not
model scent diffusion. Touch, held-object contact, ground contact, heat, and
injury are independent of visible object slots. Existing expressed sensor gains
calibrate this profile. Source-specific chemical and temperature descriptors are
contact observations rather than remote chemical identification.

Absolute position is neutralized in this profile's GPU neural body payload and
both predictor input/successor vectors. Authoritative frames, physics, rendering,
and receipts retain true coordinates. Velocity and orientation remain available.
Older profiles retain their existing semantics.

The 96-value memory query previously rejected profile 3 and could not represent
Look correctly. Formerly reserved lanes 83/84 now identify Look/terrain vision;
85–95 remain reserved. Existing valid legacy feature bytes and offsets remain
unchanged. Terrain candidates can be encoded, validated against their final
frame, and serialized through the existing query contract.

Focused verification covers opaque/transparent sight, terrain depth/collision,
targetless motor translation and retained gaze, invisible smell/contact,
candidate limits, memory round trips, and GPU-upload coordinate neutralization.
Legacy object-slot and memory-query checks protect existing profile behavior.
GPU-runtime compilation verifies predictor integration. No training or founder
promotion is part of this repair.

## Remaining integration and later groups

New-game and care-training defaults still use GroundedObjectSlotsV1. The trained
founder binds that profile in its foundation recipe. Adaptation must explicitly
bind a new foundation identity/profile and handle its optimizer state; do not
silently relabel the old founder or claim these senses are already learned.
The user's chosen path is adapting existing weights, then varied food-location
navigation, with food seen before it becomes hidden in the first wall lesson.

Visible head/gaze feedback remains unverified and belongs to group 5. Use Astra
for animation/model changes as requested. No renderer/model work was performed.

Carry these findings into later reviews:
- Group 3: trace Inspect/Vocalize outcome valuation before concluding that
  diagnostic homeostatic deltas become actual biological reward.
- Group 4: motor-bundle ambient mating opportunity still uses 1.0, while the
  passive tick uses inherited mate_response. Review this mismatch.
- Feeding: review reach/obstruction and carried-object ownership semantics
  before changing them. Distance alone does not prove a legal meal.

Groups 2–7 have not yet received this sequential end-to-end review. The biological
10–30 minute food-free survival gate remains acceptance work; it is not a lesson
to reward extended inactivity.
