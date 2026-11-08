# Grounded opportunities for the next drive circuits

Source audit against accepted main `10eecff46329d74405a8a8ab25f0e4a4c5fb60b1`. This is a source-based design constraint, not a measured autonomous outcome. The detached routing experiment establishes missing or weak neural responses; this note identifies existing physical opportunities before adding weights or world rules.

## Existing action and sensory contract

ObjectSlots enumerates per-observed-object Inspect, Approach, Flee, Eat, Grab and Play proposals. Their enumeration does not certify legality or benefit. Play is action 213, family Other; Grab is family Contact. Idle, Rest, Vocalize and explicit no-action choices also exist. Target entity IDs and positions are execution transport, not privileged semantic labels supplied to the decoder.

The grounded 24 lanes represent bearing/distance, relative velocity, color, material, shape, chemistry, contact, observer proprioception, temperature and terrain. Legacy constants with similar lane names must not be used to interpret this profile. Default peers have chemical[0] roughly 0.25–0.45, food roughly 0.75–0.95, and toys zero chemistry plus distinct color/material/shape cues. These are observable descriptors, not guaranteed universal classifiers. A single positive smell coefficient cannot selectively favor the peer band over stronger food smell.

For the same object, Play and Grab have identical physical vectors but different decoder families. Vocalize shares Other with Play, yet uses targetless intrinsic inputs (including lanes 22/23), rather than an object descriptor. Ordinary ObjectSlots does not enumerate separate Close/Release choices: Grab remains state-dependent, including release of the same held target. This distinguishes a meaningful existing neural input from the separate grip-profile research.

Sources: `crates/alife_world/src/candidate_enumerator.rs`, `crates/alife_core/src/grounding.rs`, and `crates/alife_world/src/grounded_sensing.rs`.

## Loneliness: approach a peer, then remain nearby

Real close body contact with a live peer produces SocialContact through both registered action intervals and stationary authoritative intervals. The inherited chemistry reduces loneliness; canonical founders enable the relevant developmental expression. Approach/stay is therefore a truthful opportunity. Grab cannot acquire an Agent, so routing loneliness directly to generic Contact would not fix social motivation.

Next control: distinguish real peer descriptors from food, toys and nonliving objects; measure actual reachable peer-touch receipts and loneliness change, including blocked reach, peer departure and stationary contact. Do not expose compatibility, fertility or hidden world labels as ordinary sensed facts.

Sources: `headless.rs` peer-touch helpers, registered action completion and stationary interval body-event calculation; `biochemical_graph.rs` SocialContact receptor/emitter construction; `new_game.rs` canonical graph assembly.

## Boredom: approach a toy, then actually Play

Successful reachable Play on a Ball or ActivityToy emits PlayStimulation. Canonical boredom is species 22, bound to DriveExtension0/lane 9; Awake raises it and PlayStimulation reduces it. Curiosity has a distinct novelty/investigation circuit. Inspect is not a substitute for demonstrated boredom relief, and Play must not manufacture food or praise.

Whole-care6 has no active boredom route and zeros its Other decoder readout. A candidate should make a toy-conditioned Play proposal and appropriate distant approach, while retaining real possession, reach and manipulation checks. Other contains more than Play, so a family label alone is not enough to establish the selected operation.

Next control: actual candidate vectors and selected action identity; toy versus food/peer/obstacle; distant versus reachable; failed Play; boredom and curiosity separately; final physical stimulation and body costs.

Sources: `headless.rs` Play execution; `biochemical_graph.rs` `with_boredom_circuit` and novelty/investigation construction; `new_game.rs` canonical assembly.

## Temperature: ground withdrawal in actual exposure

A surface temperature descriptor alone does not establish harmful contact exposure in the ordinary world. The explicit thermal-contact research factor adds conductance-dependent contact exposure, passive relief away from exposure and persistence of harmful exposure during sleep. Both excessive heat and cold can harm, so a signed linear temperature preference is insufficient. Conductance is not present in the current slot vector; uncertainty must remain explicit.

Thermal escape should be tested against actual ambient/contact exposure receipts with neutral, insulating and inaccessible controls. Do not reward arbitrary Flee merely because temperature stress is high. Keep the world thermal factor and neural weight factor separate.

Source: `headless/thermal_contact_candidate.rs` and ordinary body/biochemical advancement.

## Reproduction: opportunity and readiness, not distress

Nearby compatible peers can create mating opportunities only under physical reach, habitat authorization and foundation compatibility. Readiness additionally depends on puberty, health, reserves, ATP, hormones and fertility. No explicit mating action is enumerated. A peer-like appearance cannot prove those hidden conditions. Reproductive readiness must not be assigned a negative welfare objective simply to force its value down.

Sources: `headless.rs` mating opportunity/conception paths; `biochemistry.rs` reproductive readiness and eligibility.

## Rest: measure transaction biology, not advertised deltas

Rest supplies a SleepRecovery body event, while movement can cancel sleep recovery. Canonical waking recovery retains fatigue reduction but removes the direct SleepRecovery-to-ATP boost and changes sleeping metabolic rate. Rest supplies no nutrients. Legacy advertised observation deltas are not authoritative registered biology. Compare exact transaction biology after the actual selected bundle, plus reserve and organ costs, rather than promising a fixed ATP refill.

Sources: `headless.rs` body-event construction and action completion; `biochemical_graph.rs` `with_waking_recovery`; registered `biochemistry.rs` advancement.

## Implementation order

1. Repair the measured motor operating point with matched readout/tonic factors.
2. Test separately wired ATP/fatigue/hunger competition, preserving legitimate rest.
3. Add object-conditioned boredom/Play and loneliness/peer-approach routes using truthful observations and measured consequences.
4. Test thermal world and neural factors separately before claiming thermoregulation; keep reproductive opportunity semantics distinct.

The reference method remains [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), especially experiments 4–6. Each A-Life revision needs its own protected baseline, source/asset identity, signed dynamic measurements and actual-world controls; adding a path is not proof of useful biological behavior.
