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

Apply food-toy-followup.patch (SHA-256 92f53502917e153419169bfb87a9b04086fd1ed1336dbdd14d4c3b9748ae45f3) after natural-followup.patch. Exact tests ending additive_food_toy_original, additive_food_toy_candidate and additive_food_toy_sham each passed on CPU, with 268 other tests filtered out. The three arms were separate fresh hosts with the same fixed seed.
