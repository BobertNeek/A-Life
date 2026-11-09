# Why the novel toy did not yet stimulate Play

## Measured result and exact source

[The six-interval controlled Play qualification](https://github.com/BobertNeek/A-Life/actions/runs/42699800420) passed once at activation ab032b59d7bbe213387adae8ea9e86181a5eb85d. Its Play intervals had investigation 1, stimulation 0 and perceived novelty 1. The matched Play and NoManip treatments ended with the same boredom. The world accepted Play and delivered AwakeRest; this was not useful stimulation or an autonomous policy result.

This diagnosis checks the exact qualified [source manifest](https://github.com/BobertNeek/A-Life/blob/849348efcd1191e2a69d96b33be15a4ecbace37d/docs/experiments/m2-channel-native-play-recovery-list-v5/source-manifest.json) and [107-file source capsule](https://github.com/BobertNeek/A-Life/blob/849348efcd1191e2a69d96b33be15a4ecbace37d/docs/experiments/m2-channel-native-play-recovery-list-v5/host-source-capsule.tar.gz), SHA256 073edda7bf4949c6bb146a836be23d8832e5f2552afd06b1881cd86de3de99af. It performed 410 source-identity/arithmetic checks, with zero constructors, restores, native forwards, world/biology intervals or source edits.

## The source-supported mechanism

1. Canonical preparation computes each tracked object's novelty from retained target-bound familiarity. Familiarity is 1 - 1/(1+n), where n counts retained Look, Inspect, Interact or Hold observations for that owner, profile and object. No record gives novelty 1; one qualifying observation immediately gives novelty 0.5. There is no minimum waiting time. Sources: alife_runtime/src/canonical_preparation.rs:678-688,761-785 and alife_core/src/memory.rs:917-958.

2. The channel-native compatibility metadata first prefers a selected posture that is not a channel member, such as Idle/Gesture. AwakeRest is a Rest channel member, so this fixture instead falls back to the first selected physical row: mandatory NoLocomotion, action 103, Move/Approach, target NONE. It does not choose Play or AwakeRest as its episodic key. Sources: channel_native.rs:148-154,179-184,258-263; motor_mediation/confidence/policy.rs:69-112; recovery_operation.rs:29-58.

3. The shared host's DecisionSnapshot/finalized memory query uses that one metadata index. The resulting targetless Move record cannot contribute to the toy's familiarity, even if the interval otherwise becomes memorable. The actual toy target is in the physical Play channel, not the sole legacy memory key. Sources: grouped_host_loop.rs:466-473,503-536,766-775; experience.rs:928-973; memory/candidate_recall.rs:15-104.

4. Physics nevertheless receives the actual Play target's predecision novelty correctly. For a completed target interaction, the novelty wrapper sets investigation to novelty and scales stimulation by 1 - novelty. Novelty 1 therefore produces investigation 1 and stimulation 0. Sources: alife_world/src/headless.rs:4993-5020,5880-5940.

This explains persistent novelty in the controlled pattern. It does not imply that a creature could never acquire familiarity through a separate target-bound Look/Inspect experience. Extending this exact pattern until stimulation appears is not the supported repair.

A second source limitation is separate: merely novel Completed Play/Interact Touch is not an unconditional meaningful-memory event. Other consequence/prediction thresholds could admit it, but the safe V5 result does not expose those values. Do not claim a toy-keyed Play record was actually rejected by that gate, because this fixture never produced such a key. Sources: memory/candidate_recall.rs:683-712.

## Neural input is a separate question

The host retains object novelty and uses it for attention/physics. The source sensory receptor mapping covers sensory lanes 0-39; flat novelty lane 41 has no direct receptor. Grounded candidate lanes 44-45, reserved for personal novelty/play expectancy, remain zero. Physical object rows, body/homeostasis, recalled context, cognitive projections and current drives still reach the controller. These are source facts, not a measured neural ablation or proof of complete novelty blindness.

A contact-attribution correction should not silently add those neural inputs or change the inherited sensor ABI.

## Proposed fresh-owner repair, not yet qualified

Implement a separately owned bounded retained-contact evidence bank. At a successful once-only close, require exact command/candidate/frame/source/bundle/receipt binding and genuine completed qualifying target contact. Exclude preparation, failure, incidental collision, release/place and NoManip. Deduplicate by owner/profile/source sequence/target across channels, retries and representative overlap. Preserve rollback, cursor/seal ownership and byte-exact cold restore; forget when retained records are evicted.

Each contact is familiarity evidence with explicit UnknownJointValue semantics. It must not create an isolated Play reward, family advantage, target latent, biochemical delta, or second synthetic joint ExperiencePatch. Preserve the original representative/value/replay path once.

A conservative fresh-owner novelty rule can use max(legacy familiarity, contact familiarity), not an unproved sum or exact union. This prevents double counting overlap but can undercount disjoint records; document that tradeoff. Capacity, encoded/resident/index bytes, allocations and actual reads/comparisons/writes must be measured. Admission must reserve an explicit bounded cost before physical dispatch, and actual work/debit must be consistent. Proposed capacity or operation ceilings are design budgets, not measured CPU energy or neuron counts. Existing owners and their work receipts remain unchanged.

## Fixed next qualification

After implementation and source review, freeze one new fixture: one common Play+AwakeRest interval, then warm Play, identical cold Play and matched cold NoManip from that boundary. Total: four source/world/biology intervals, one admitted fresh host, two cold restores, zero optimizer updates or extra probe forwards. Include all internal constructor/phenotype validation work separately.

If one real contact is retained, the next supported completed Play predicts object novelty 0.5 and stimulation 0.5 at intensity 1. Report actual contact, original metadata, retained count/digest, novelty, stimulation/investigation, memory work, recovery, movement, energy and boredom. Require exact warm/cold continuation and unchanged actor/optimizer/private banks. Do not require a benefit, force a candidate, change a seed, extend the horizon or add an object after seeing the result.

This is a proposed new research owner, not migration of an old life or promotion to defaults. The original [weight-space-programming reference](https://rikiyat.github.io/blog/weight-space-programming/) remains a method reference; fixing an information bottleneck does not by itself establish new competence.
