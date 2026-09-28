# Next founder run: terrain vision, small mazes, and grounded vocabulary

Status: design only. No training, demonstration collection, pilot, or scheduled launch is authorized by this document. The user wants the next run overnight and wants understanding and speaking given equal priority. Launch time will be settled separately.

This continues the [care training regimen](n2048-care-training-regimen-2026-09-21.md) using the existing production world, founder trainer, chemistry, replay, and checkpoint paths. It is not a second trainer or architecture. The v2.0 requirements AOA-FOUND-002/004/007/008, AOA-TEACH-004/005/008, and AOA-BIO-024 apply.

## What we want to see afterward

A fresh founder looks for food using its 220-degree view and independent head movement, respects opaque terrain, navigates a small unfamiliar maze, recovers from a dead end, and eats without teacher guidance. It also associates a modest vocabulary with objects and actions: it considers requests such as `look food` or `approach toy`, and can produce a relevant noun or short action description itself. Requests remain sensory evidence for the brain, not compulsory commands.

The language target follows Creatures 3 / Docking Station: category nouns, simple verb+noun phrases, and eventually short descriptions of needs. Docking Station documents category naming, requests, action descriptions, and need expressions; it also explicitly allows creatures to ignore requests. We are adopting that style, not promising its entire dictionary before equivalent agents exist here. [Official Docking Station manual, sections 3.3 and 3.5](https://cdn.akamai.steamstatic.com/steam/apps/1659050/manuals/DSManual_final.pdf?t=1721031581).

This run is not intended to solve large arbitrary mazes, prove general conversation, or certify the 10–30 minute food-free biology requirement. Those remain separate acceptance questions.

## Starting point and gaps found in the code

Use `assets/founders/terrain-care-n2048-v1` as the unpromoted source, or a separately verified terrain successor derived from it. This is the adapted old N2048 foundation, preserving its weights while changing the sensing profile and resetting incompatible optimizer/value state. Do not start from the old object-slot overnight checkpoint directly, the 16-decision coordinator check, or a saved adult's personal memories.

The September 27 player session demonstrated touch and autonomous consumption of player-placed food. It did not demonstrate maze navigation, retained language learning, or speech competence. Available care receipts also do not yet verify praise delivery or accepted play.

Source review at `25454d4` found:

- `foundation_training.rs` has four lesson kinds. Its terrain teacher remembers seen food and reads a distance fan, but its obstacle scenario is one closing blocker, not a maze with branches. Retaining a food direction alone cannot tell it how to escape a dead end.
- Walking is now approximately 0.1 world units per ordinary tick before biological scaling. Demonstrations collected before that correction have unsuitable timing; regenerate them against the final source.
- `foundation_training_warmup.rs` requires exactly 32 demonstrations, eight per existing lesson. That fixed category accounting cannot describe this curriculum. Keep bounded storage, but make the existing manifest express lesson counts and multiple training windows.
- Warmup, cycle, and pilot trainable masks explicitly exclude `SpeechPayload`. Care imitation targets contain action choices, not speech targets. Unfreezing a mask alone will not teach word production.
- The lower-level trainer already has speech targets and WGSL speech-gradient kernels. Reuse those, but its synthetic speech mechanics exercises are not grounded vocabulary demonstrations.
- `alife_school::LanguageNursery` uses real spatial speech but currently requests the old object-slot profile and is not connected to the care founder campaign. Update/reuse its exposure path rather than introducing a parallel nursery.
- The bounded translator assigns unfamiliar surface words to alias tokens, resolving collisions against existing bindings. The UI starts with an empty binding list. Training and gameplay therefore need the same explicit surface/token convention and verified persistence; a token number or a rendered English word is not evidence that the creature knows its meaning.
- `PpoJointAction` accounts for action/channel choices, not generated speech-token probabilities. Do not treat it as a complete speech RL objective.

## Repairs required before this run can be launched

1. Extend the existing scenario configuration and campaign lesson selection with vision search, maze navigation, vocabulary reception, and vocabulary production. Build walls from world-authoritative geometry used by both occlusion and collision. Validate reachability and traversable corridor width before accepting a layout. Rotate, resize, and vary actual junction/dead-end structures, not just the old blocker's position.
2. Upgrade the current navigation demonstrator with observed junction memory, failed-route bookkeeping, and backtracking. Keep the existing perception/candidate interface. The teacher must distinguish body heading from head direction and use measured displacement, including zero displacement after a block. Its demonstrator-only memory is never copied into the founder.
3. Connect ordinary spatial utterances and observable referents/actions to source-bound replay in the current warmup/cycle pipeline. Add speech supervision on real production neural context, including token bits, speech act, emit/stop, and sequence timing. Existing single-output speech targets may need a bounded multi-output representation; do not fake a phrase by displaying target text.
4. Train speech-related inherited routes and decoder weights with that grounded auxiliary objective. PPO continues to train legal action choices, including choosing to vocalize. Speech-token RL can remain deferred until its sampled probability and credit are represented correctly. Do not clamp internal speech neurons as inputs or use a teacher-selected payload during evaluation.
5. Share and persist the bounded vocabulary surface/token bindings through the existing translation path. Pin their digest in corpus, checkpoints, and evaluation receipts. The mapping translates words into symbols; learned associations must live in neural weights and ordinary lifetime learning. A newly spawned founder gets fresh personal records, not the teacher's lexicon ledger or episodic memories.
6. Make episode limits distinct from gradient-window limits. Preserve a life and its memory across 256-decision windows; the existing 128-decision burn-in is the starting configuration. Split long demonstrations too, with real preceding context. Do not reset a maze whenever a training window ends. Preserve actual world-time returns across natural sleep and terminal-death handling.

The existing process lock, source checks, disk/RAM limits, exact export/rebind, and finite-value checks stay in charge. Source/profile/schema or mask changes invalidate matching preparation evidence and optimizer compatibility as appropriate; do not reuse stale preparation solely because a filename exists.

## Curriculum and episode lengths

Navigation and language start together in easy scenes. Each stage retains easier lessons so gaining speech or maze skills does not erase feeding. These limits are initial engineering settings, not measured optima.

| Stage | Navigation experience | Vocabulary experience | Initial waking-decision limit per life |
| --- | --- | --- | --- |
| A: see, turn, act | Food and toy at varied bearings, including behind the body; head turns reveal them, body turns align walking. Opaque and transparent blockers distinguish visibility from reachability. | One-word category naming and action naming. Hear and produce words equally. | 128–256 |
| B: retain and detour | See food first, then lose sight behind a corner or occluder. Choose clearance, go around, reacquire, eat. | Contrast `look food` with `look toy`, and `look food` with `approach food`. Short truthful noun/action utterances. | 256–512 |
| C: small mazes | Two to four junctions, alternate routes, one or two dead ends; learn to reverse a failed choice. Start with seen-food goals; add fully hidden-food search only after the earlier skill works. | Understand and produce short phrases in varied layouts; include manipulation once its receipts qualify. | 512–1,024 |
| D: transfer | New topology families, distractors, varied corridor dimensions and transparent barriers. Routes generally require 8–20 world units of travel, with limits allowing biological slowing and mistakes. | Hold out some valid verb+noun combinations and object exemplars. Fade prompts and check spontaneous relevant speech. | 1,024–2,048 |

At 20 world ticks/second, 256 awake one-tick decisions are only about 12.8 simulated seconds; 2,048 are about 102.4 seconds. Natural sleep changes the relation between waking decisions and elapsed simulation time. Record both. Route length and measured locomotion set episode feasibility, rather than insisting that every maze fit into 32 ticks.

Begin with 40% navigation lives, 40% vocabulary lives, and 20% feeding/care rehearsal. Half the vocabulary allocation is reception and half production, with comparable useful examples and separate loss/behavior measures. Navigation lives can contain language exposures, but also include silent lives to measure independence. Preserve natural fatigue and sleep; don't manufacture recovery quotas or reward extended inactivity.

World seed, food position, spawn position, orientation, object order, and topology vary. Keep fixed held-out topology families and seeds outside training and demonstration generation. Mirrors and rotations alone are insufficient. Some sight lessons place food outside useful local scent range; other lessons allow normal scent assistance. Do not globally turn off smell, provide exact source coordinates, or call scent-guided success proof of vision use.

## Vocabulary for this run

Start with categories the game can ground:

- Nouns: `food`, `toy`, `creature`, `obstacle`. Use more than one object and placement for each. Obstacles include the maze's physical blockers; no noun class label is injected into sensory channels.
- Verbs: `look`, `approach`, `retreat`, `eat`, `get`, `rest`. `Get` requires the existing Grab receipt to show actual carrying. `Rest` is taught in brief naturally tired contexts, never as the answer to hunger or a standing inactivity objective.
- After core words work: `hungry` and `tired`, paired with separate real interoceptive states. These are descriptions, not extra reward chemicals.

Keep `push`, `pull`, `drop`, and `drink` outside the first set until their gameplay executor and sensed outcomes are qualified for training; Creatures having a verb is not evidence that this game can ground it. Likewise, creature names, Hand naming, query words, negation, and the wider C3 category list can follow this first functional set.

Reception: present an unambiguous sensed referent, say its category through normal hearing, pair the verb with a real action demonstration or source-bound offline action example, then remove assistance and make a request. Introduce two-object choices early. For `look`, the head must reveal/fixate the requested sensed target rather than move directly toward it. For `get`, carrying changes; for `eat`, food is consumed; for `retreat`, separation grows.

Production: train a noun while its referent is sensed, and a verb/action phrase while the corresponding action is actually being performed. An emitted `approach food` must agree with the neural action and available evidence. Begin with one word, then two; don't reward a monologue every tick. Include unprompted speech opportunities, silence examples, and changed scenes so echoing the last heard word is insufficient. A fluent HUD translation never substitutes for the GPU's literal output.

For words alone and later combinations, vary which target is nearer, left/right placement, hunger, and requested action. Paired assessment scenes keep the body and layout the same while changing the noun or verb. Compare with silence. Reserve some combinations, such as `look creature` after learning both words separately, to check association rather than memorized phrases. In a hidden-target scene, hearing a noun does not create a hidden target candidate; the brain must search.

## Teacher behavior: cause and effect

The navigation teacher first scans using head movements. A food sighting establishes a remembered relative goal. Before stepping, it evaluates clearance in the intended body direction; a sideways head turn is not permission to walk through a wall. It aligns the body, steps, and updates its estimate from actual displacement.

At a newly observed junction it records locally observed exits and which have been tried. A promising untried exit is explored. If a route closes or reaches a dead end, it turns around, returns along the observed route to the last junction, marks that exit exhausted, and chooses another. Seeing food again refreshes its estimate; contact enables ingestion. A bounded history overflow or unresolved loop fails the demonstration rather than teaching repeated spinning. Full-layout shortest paths are permitted for scenario reachability and assessment metrics only; they are not teacher or learner sensory inputs.

The language teacher names observable things, demonstrates feasible actions, waits for the actual receipt, and then offers a new opportunity. Early offline imitation gives gradients from grounded action/speech targets; it is explicitly founder pretraining, not a live teacher writing creature intent. During learner-driven collection, teaching influences the creature through hearing, visible behavior, objects, and normal care channels only. Ambiguous references and failed demonstrations are not marked as understood.

Food yields the existing gene-controlled hunger relief and nutritional consequence. Actual injury produces its existing pain response. A harmless blocked attempt produces mild disappointment, not severe pain. Correct language behavior can earn a bounded ordinary praise stimulus through the existing world-to-chemistry path, after delivery is verified. The teacher does not write a floating-point reward directly. Apply a cooldown and finite praise opportunities per lesson so repeating words or repeatedly bumping walls cannot farm reinforcement. Successful speech counts do not themselves become a new biological reward.

Do not silently add navigation progress bonuses based on hidden food distance or the true maze solution. Offline imitation supplies a useful starting policy; teacher-free reinforcement supplies consequences. Route length, word accuracy, and task completion are evaluator measurements, separate from the creature's chemical credit.

## Overnight execution shape, once separately authorized

Propose an eight-hour hard deadline, matching the previous campaign. Preparation and supervised warmup happen before that deadline when separately authorized. Do not assume the existing 160-epoch setting is best: use a small fresh corpus spanning the new causal mechanisms, stream windows, and stop warmup when reserved demonstration performance plateaus rather than grinding the same tiny corpus all night.

During the campaign, use one GPU lane on the RTX 3050. Cache immutable phenotype data, reuse the device, stream compressed replay, buffer compact receipts, and prebuild bounded CPU layouts. Do not run the graphical game alongside training. Existing serial cohort execution is proven; fused multi-world training is not. Parallel CPU work is useful only if it leaves simulation and GPU authority intact and measured resource limits hold.

Stages advance by behavior, not an hourly timer. Suggested promotion floor: six of eight held-out routes completed within the stage budget, and six of eight paired language opportunities correct in each of reception and production. Track the two language skills separately so one cannot conceal failure of the other. Small panels are screening evidence, not a statistical guarantee. Stages C/D also require at least one genuine dead-end recovery, and production requires context-dependent output beyond echoing. If a gate fails, remain at the current stage and mix in the failed mechanism; do not unlock harder mazes simply because time passed.

Run these compact panels at a candidate stage transition or a materially changed checkpoint, not after every cohort. Reserve the final approximately 30 minutes for frozen evaluation of the best compatible checkpoint. Compare navigation, reception, production, and basic care separately; do not select a founder by one aggregate training score. A checkpoint that regresses care or either language direction is not automatically better because it ate more in familiar training worlds.

Keep latest and best compatible sealed checkpoints, actor/value/optimizer state, bindings, source/profile digests, stage, world seeds, elapsed time, meals, blocked attempts, collisions, head/body turns, junction reversals, received tokens, emitted tokens, and real chemical credit. A broken export/rebind or nonfinite update stops the campaign. Routine unchanged monitoring stays quiet. Weight merging remains an experimental follow-up, not a default operation in this run.

## Minimum meaningful validation

Before launch, reuse existing checks and cover only the distinct new failure mechanisms:

1. One causal navigation sequence covering head/body distinction, opaque occlusion, legal movement, a dead end, backtracking, reacquisition, and consumption. Include a transparent blocker variant in the same focused check. Inspect demonstrator receipts, not just success totals.
2. One language sequence covering real hearing, the correct profile, a changed-noun/changed-verb pair, literal GPU speech with correct emit/stop, praise delivery, and a silent or changed-scene counterexample. Check actual gradient reach into the intended routes and speech decoder. No test per word.
3. One combined export/resume case showing speech weights, vocabulary binding identity, and continuous maze memory survive their respective foundation/checkpoint paths. Fresh founder creation must retain trained weights while starting with fresh personal records. Validate PPO action likelihood separately from the auxiliary speech objective; don't invent token likelihoods.

Run relevant source-bound production/replay parity preparation once after the repairs. Frozen learned-behavior panels are necessary learning measurements, not a request for a broad new unit-test suite. Use the existing `--evaluate` path after extending it to these scenarios, with no demonstrator, optimizer updates, corrective prompts, or SLM assistance; include the test request itself for reception. Frozen means the inherited foundation receives no external gradient update, not that working memory or normal lifetime learning is disabled. Start fresh individuals and report first-attempt behavior separately from improvement within a life.

After the overnight result, a short 1x player session must demonstrate useful searching, food consumption, understandable requests, and actual creature speech. The separate wall-time food-free biology gate remains required before promotion. Neither test should train the creature to spend the game doing nothing.

## Current stop point

This document is the proposed run and repair specification. Those new maze/language integrations have not been implemented or qualified. No training or overnight automation was started while preparing it.
