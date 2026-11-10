# Additive contact and Play-drive experiment

On 2026-11-10 I combined the previously qualified Play-need runtime source with the actual-contact Phase B source in a temporary CPU build. This is an experimental integration, not a production change or trained-brain migration. Main and acquired checkpoints were not modified.

## What ran

The combined code compiled with Rust 1.99.1 using offline locked dependencies. First, the existing contact fixture and the separately owned Play-host cold-continuation fixture both passed in the combined source. Those two checks used separate hosts.

I then made the controlled contact fixture select one of three native recipes: Original60, Play Candidate76, and equal-work Padded Sham76. Each fresh host performed one scripted Play, then a warm Play, a matching cold Play restored from the prefix, and a matched cold NoManip continuation. All three exact tests passed on CPU. Each arm performed four world/biology intervals and two cold restores; the warm and cold repeat-Play outputs matched in each arm. The existing rejection audit was run on each authentic first receipt.

In all three arms, the first toy interaction produced investigation but no stimulation; the next scripted Play had novelty 0.5, produced stimulation 0.5 and reduced the small accumulated boredom to zero. The matched NoManip continuation had no stimulation and boredom increased. Candidate and equal-work sham had the same observed biology in this low-boredom, forced-action case. They cost more body energy than Original60 in this fixture, as expected from their added work. No optimizer or lifetime weight updates occurred.

## What this proves and does not prove

This is a first narrow additive host test: the Play-drive recipe and contact-familiarity mechanism coexist with awake rest and save/restore in one controlled creature. It does not show that Play is selected autonomously, that the candidate is better than the equal-work sham, or that care, grip, learning and other skills work together. The boredom level was far below the candidate's positive-drive reference; these observations cannot assess its intended high-boredom action preference. Full compute pricing, GPU hardware, all checkpoint cross-owner negatives and a real long-lived creature remain untested. The contact production dispatch remains closed; the fixture uses a test-only controlled strategy.

## Reconstructing

Start with the exact 830-file Phase B source described in [the source checkpoint](../contact-phase-b-source-v2/README.md), then apply [the test-only contact permit](../contact-phase-b-local-test-v1/test-permit.patch). Apply the published [Play runtime patch](https://github.com/BobertNeek/A-Life/blob/qualification/m2-play-hosted-qualification-v3/docs/experiments/m2-play-hosted-qualification-v3/OWNED-RUNTIME.patch) (SHA-256 61fbd9e2f2ce4319ab223d52977e1355b74317f2dbba5c9f8edcdaea264f1636) with git apply --reject. The expected rejects are two grouped-host-loop hunks and three optional-vocal-host hunks. Then apply integration-resolution.patch (SHA-256 02279fcbf3b389de7b9602281903c6335d5a66becb71758399d5b5f001da28ae), which resolves those conflicts, corrects the Play test imports, and defines the three test recipes. I verified that applying the resolution reproduced all seven changed files byte-for-byte.

Run the exact lib tests whose names end in additive_original_contact, additive_play_candidate_contact, and additive_play_sham_contact. Each passed individually, with 262 other tests filtered out. The earlier combined-source Play host continuation and contact fixture also passed individually. These focused results are not a full workspace test.

## Unforced third decision

After two scripted interactions with the same toy, I let each fresh host make one unforced third decision. Original60, Candidate76 and Padded Sham76 all selected the same Play channel on that seed. In each, Play completed and produced stimulation 0.6666666, alongside an Avoid action and another channel. The fixture checks the Play channel outcome, not overall joint success. Each test executed exactly three world/biology intervals, of which the first two were scripted, and did not update founder weights. One seed and one natural decision per arm are not evidence that the candidate caused the choice, that it learned, or that it generalizes. Boredom was low, below the added circuit's positive reference.

Apply natural-followup.patch (SHA-256 bf0dae8ecbec917ca4f45ced9a975cef8e6e0f78b2992ce4ad3cffd869de67c8) after integration-resolution.patch. Run exact tests ending additive_natural_original, additive_natural_candidate, and additive_natural_sham; all three passed on CPU. The first build of this follow-up failed because the test tried to serialize a non-serializable world receipt; the corrected test reports selected channel primitives and measured stimulation directly.

## Unscripted food-and-toy scene

A later CPU fixture used a fresh ordinary physical scene with one reachable root and one nearby toy. No actions or drive levels were forced. It ran one seed for 32 host calls in each of the Original60, Candidate76, and equal-work Padded Sham76 arms, with the contact-familiarity code present and synaptic training closed. Each arm completed all 32 calls and consumed one 0.65-nutrition root, first receiving nutrition at call 16. Each had 16 Play selections, first stimulation at call 8, 23 successful joint bundles, and total reported stimulation about 6.571. The recorded action channels and displayed outcome fields were identical across the three arms after excluding the arm label. These records did not include all probability vectors or full body/optimizer states, so this is not a claim that all underlying states were identical.

This demonstrates one short instance of unscripted feeding and play coexisting with the contact mechanism. It does not demonstrate a Play-circuit advantage: candidate and equal-work sham behaved alike in the measured fields, and the initial boredom was low. It does not establish retained learning, multiple-seed reliability, GPU behavior, mature-organism care, or a complete skill suite. No long-lived or trained checkpoint was imported, modified, or promoted.

Apply food-toy-followup.patch (SHA-256 e8d1c62cdb94b5d8bfb6b8fb4f057ce8e16ecea01a5f3b7e80d3ee8d60e42680) after natural-followup.patch. Exact tests ending additive_food_toy_original, additive_food_toy_candidate and additive_food_toy_sham each passed on CPU, with 268 other tests filtered out. The three arms were separate fresh hosts with the same fixed seed.

## Two additional seeds

I repeated the same unscripted 32-call food-and-toy fixture with two further seeds, each on fresh Original60, Candidate76 and equal-work Sham76 hosts. All six tests passed. For seed 2, each arm consumed 0.65 nutrition at call 8, first produced Play stimulation at call 2, and reported 18 successful bundles. For seed 3, each consumed 0.65 nutrition at call 11, first stimulated at call 3, and reported 20 successful bundles. Within each seed the recorded outcome fields matched across arms; this does not assert identical unrecorded probability vectors or complete internal states. All three short seeds therefore show food and Play coexisting, but no demonstrated behavioral advantage of Candidate76, no learned-skill claim, and no long-term or GPU conclusion.

After food-toy-followup.patch, apply multiseed.patch (SHA-256 70055f6f2f99b766166ff182331c0e4319c134e146adfe3a7e0f8284). Tests named additive_food_toy_{original,candidate,sham}_seed2 and _seed3 passed individually. During reproduction verification I found the original food-toy patch contained one extra blank context line, corrected it, and verified both patches apply in order and reproduce the tested source byte-for-byte. The earlier immutable commit retains the original flawed patch; use the latest version.

## 256-step unscripted comparison

I ran one fresh host per arm in the same food-and-toy setup for 256 normal host calls, using the first fixed seed and unforced actions throughout. All three tests passed on CPU. Each arm received one 0.65 nutrition event and about 7.4801 total Play stimulation; each completed Play 10 times and ended at health 1.0. The Candidate76 arm had 59 Play attempts that remained in Attempt phase, versus 75 in equal-work Sham76 and Original60. It selected NoManip 79 times versus 69; its joint-success count was 95 versus 85. This distinction matters: the additional successes are not ten extra completed meals or successful Play events. Candidate, sham and original all ate at call 16 and produced the same total stimulation. Candidate and sham started from the same seed and paid the same appended recipe work, but their subsequent trajectories can diverge.

These are one seed, 256 host calls, no training updates, and no GPU execution. The recorded fields do not establish that the Play circuit improves overall welfare, learned competence, survival or care. Final reported hunger was approximately 0.33250 for Candidate76 and 0.33255 for Sham76; final body energy approximately 0.855020 and 0.854929 respectively. These small differences should not be interpreted as a measured general advantage. The specific reason for fewer failed attempts needs targeted confirmation.

After multiseed.patch apply long-followup.patch (SHA-256 bbee93b9cc14c1f58e3fa3e9eb78c6325dd44918ea5c8a95626273e41cbcf572). Tests additive_long_candidate, additive_long_sham and additive_long_original each passed individually. The patch chain was re-applied and the resulting test file matched the exercised source byte-for-byte. Exactly 256 source/world intervals were recorded per arm, with no founder optimizer updates.

## Second 256-step seed

I repeated all three fresh-host arms for 256 unforced calls with the second seed used in the short tests. Each consumed one 0.65-nutrition root, completed Play 8 times, produced about 5.604365 total stimulation, and finished at health 1.0. Candidate76 had 66 incomplete Play attempts and 79 successful joint bundles; equal-work Sham76 had 77 incomplete Play attempts and 73 successful bundles. Original60 had 77 incomplete Play attempts and 73 successful bundles. Candidate selected NoManip 69 times against 63 for each control. These extra successes are not extra meals or completed Play. Final reported energy was approximately 0.854599 in candidate and 0.854544 in sham. This is a second short seed in the same scene, still insufficient to establish a general welfare or learning advantage. Synaptic learning remained closed; no acquired checkpoint or GPU hardware was involved.

Apply long-seed2.patch (SHA-256 5e6541820eb22c76214818c37b1b2908cf68df9a2be0b22ccfedbd23db566548) after long-followup.patch. The three tests ending additive_long_candidate_seed2, additive_long_sham_seed2 and additive_long_original_seed2 passed individually. I reapplied the patch sequence and verified the resulting test source byte-for-byte.

## Candidate/sham checkpoint boundary

A focused CPU test created a fresh Candidate76 contact host and saved it before any world step. Restoring that candidate payload as the equal-work Sham76 policy was rejected; the original candidate host remained byte-identical and restored successfully as Candidate76. This establishes the tested direction at the initial boundary, not every possible cross-owner, later-state, or corruption case. Apply cross-owner.patch (SHA-256 cd59b9d9fa52cbe822721fb34a4bf6f65f87dd520f356470ea27be643ef41e2e) after long-seed2.patch and run the exact test ending additive_candidate_sham_cross_restore_rejected. It passed; 281 other tests were filtered out.

## Contact sampler gradient checks

I adapted the existing channel-native synthetic sampler tests to the actual-contact owner and RNG domain. Eleven tests passed on CPU, including numerical checks of the joint score gradients, shared Idle/Quiet gradient accounting, normalized support, and proof-context validation. These checks do not update synapses and are not evidence that learning works in the combined host.

The first reused suite passed 10 and failed the counter-tamper assertion. The test assumed adjacent random counters must choose different bundles; in the contact RNG domain they can coincidentally choose the same bundle. I corrected the test to validate the counter against the known expected sampling context rather than infer tampering from the sampled result. The second run passed all 11. This was a test-assumption correction, not a demonstrated production bug fix.

After the previous integration and test-permit patches, contact-math.patch (SHA-256 0278f36e9c0b8e26c132c94835df0aa33195cda19dc8e516599123e4e705ac5c) registers the contact-specific tests. Run the exact filter cpu::motor_mediation::confidence::policy::optional_vocal::actual_contact::shared_math_tests::. The full test process reported 11 passed and 282 filtered out. The patch was checked for applicability; no production behavior was enabled.
