# Roadmap

Updated 14 October 2026. The immediate priority is the persistent creature game
described in section 1.1 of the organism and gameplay design: care, useful
learning, and save/load first; breeding and practical breed promotion next.

The [core game loop delivery plan](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/core-game-loop-plan-2026-09-20.md) contains the
current sequence, source findings, Docking Station lessons, and exit criteria.
It also sets the minimum-test rule: use the smallest relevant existing check,
combine acceptance into one evolving play scenario, and expand only for a
concrete failure. This is a plan, not evidence of completed gameplay.

## 1. Establish the ordinary production baseline

Live GPU-to-voxel projection is implemented in source. Check it inside the actual
care scenario, starting with one creature and a fresh save. Preserve existing
saves. Repair the first observed break rather than repeating a broad audit.

Exit gate:

- a real GPU-selected action changes the authoritative world;
- the matching voxel entity receives the resulting transform or visibility change;
- renderer code cannot write world truth;
- existing causal output and a short recording show the same organism through
  input, selection, world outcome, and presentation.

## 2. Deliver the first playable care loop

Validate a capable founder and connect its explicit identity to ordinary New
Game, birth, archive, and restore. Make reachable food, eating, rest, and recovery
work on the production terrain. Provide direct care tools, readable needs and
consequences, and contextual help in the player view.

Exit gate:

- care produces visible, real physiological consequences;
- one grounded lesson changes later independent behavior;
- natural sleep and fresh-process save/load preserve the same individual and
  its acquired ability;
- ordinary controls demonstrate the connected loop, not only its components.

### Controls and continuity throughout

Pause, speed, load, save, selection, follow, and new-world commands use the same
live runtime and projection. Preserve them during every gameplay change.

Exit gate:

- pause stops simulation and presentation at a sealed boundary;
- load atomically replaces runtime and scene or fails without partial state;
- save/load succeeds across a fresh process;
- controls report unavailable neural hardware honestly;
- disabled or diagnostic-only features are visibly labelled.

## 3. Deliver a small community and generational play

Extend the working loop to the default six creatures, with useful objects,
sustainable resources, readable social behavior, and measured performance.

Autonomous production lifecycle has existing world/runtime hooks. Complete the
ordinary experience of reproduction, inherited variation, development, death,
and lasting lineage records.

Exit gate:

- viable offspring have fresh identities and meaningful inherited differences;
- birth archive precedes GPU admission, and final life archive precedes retirement;
- stable IDs, GPU residents, and visible entities agree across lifecycle changes;
- restarting restores the same family and lineage;
- explicit named breed promotion retains a useful skill in a fresh descendant,
  clears personal records, and leaves the source individual unchanged.

Follow the [approved promotion scope](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/reviews/2026-09-11-breed-promotion-scope.md).
Ordinary mating must not silently copy an acquired adult mind.

## 4. Advance EI1 research separately

Treat the retained `Blocked` corpus as diagnosis, not promotion.

This research promotion gate does not block repairing a small interaction or
establishing the first playable care loop. Gameplay progress does not itself
promote EI1.

Exit gate:

- the exact source, tree, adapter, backend, config, and corpus are bound;
- intact scores are positive;
- treatment is not worse than control in every required cell;
- every minimum control margin passes;
- required plateau windows are complete and valid;
- the causal zstd sidecar streams successfully and matches every promoted receipt;
- focused gates contain no EI1 ignores.

If measured ability still fails, retain `Blocked` and improve the training design rather than relabelling the evidence.

## 5. Scale by measurement when larger profiles are the target

Keep brain class and population capacity as separate decisions.

Measure the initial supported population in the ordinary play session. Broad
population studies are separate work, not a prerequisite for the care milestone.

Exit gate for each production class and population profile:

- complete causal, learning, sleep, restore, memory, topology, and rollback evidence;
- measured p95 tick and VRAM use on the named adapter/backend;
- populations 1, 10, 50, 100, 250, and 500 measured for both sensor profiles where required;
- admission, throttling, save migration, soak, and replay gates pass;
- N4096 remains research-only until its own equivalence and rollback gate passes.

### Current performance follow-up: 50 individuals at 20 FPS

Target i7-3770K / 32 GB / RTX 3050 8 GB at 1080p, with the existing 20 Hz
simulation/cognition schedule, useful learning, individual memory and exact
continuity. **20 FPS with 50 creatures remains Unknown.** Keep the care and
small-community milestones above in sequence; a broad population study is not
an additional prerequisite for the first playable care loop.

Source-integrated groundwork, confirmed in main and the inspected consolidation
candidate (implementation status, not target-hardware or gameplay proof):

- [x] Memory-sensitive attention without the unused preliminary finalized frame:
  [`6aef08b1e8ded7691d2d5c111e9a5b3225e84c64`](https://github.com/BobertNeek/A-Life/commit/6aef08b1e8ded7691d2d5c111e9a5b3225e84c64)
- [x] Bounded production CPU preparation with at most two participants and the
  existing serial fallback: [`b24417cad91a859211ea6730948760c2582553d9`](https://github.com/BobertNeek/A-Life/commit/b24417cad91a859211ea6730948760c2582553d9)
- [x] Redundant private topology-plan/lifecycle validation and digest work removed
  while retaining transaction/rollback guards:
  [`035d76fa8101c17ec5748c60e464c0c71a169122`](https://github.com/BobertNeek/A-Life/commit/035d76fa8101c17ec5748c60e464c0c71a169122) and [`994fceac4c6777b4390dc17ed1e4a7354b76930a`](https://github.com/BobertNeek/A-Life/commit/994fceac4c6777b4390dc17ed1e4a7354b76930a)
- [x] Discarded draft/attention copies removed:
  [`51ef0d88ab946bba75c63040348cad42d6f8be04`](https://github.com/BobertNeek/A-Life/commit/51ef0d88ab946bba75c63040348cad42d6f8be04)

Remaining measurement-led work:

- [ ] Bind the actual integrated source/tree, executable, configuration, adapter,
  founder/save and workload; separately authorize the target-hardware window
- [ ] Obtain paired 8/16/32/50-population receipts with fresh and populated personal
  memory, spread/crowded placement, FPS, progressed TPS, due/completed cognitive
  rows, latency tails, catch-up debt and mutually exclusive stage costs
- [ ] Choose only the measured remaining bottleneck: exact organism-local reuse,
  allocation/layout, world queries, transfers/waits or presentation scheduling
  are hypotheses; retain grounded candidates, individual exceptions, bounded
  work, GPU-authoritative actions and current cognition cadence
- [ ] Preserve care/learning/sleep/lifecycle and fresh-process save/load in one
  evolving ordinary play scenario; retain Unknown for missing source-bound proof

The [historical optimization plan](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/performance/20261001-fifty-creature-optimization-plan.md)
preserves cost arithmetic, alternatives and falsification criteria with an
explicit current consolidation context. The [bounded experiment blueprints](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/deep-planning-experiments-2026-10-01.md)
retain matched memory and learning screens as conditional follow-ups; they do
not launch a training campaign. The [dated skill review](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/reviews/2026-10-01-deep-planning-skill.md)
records the reusable method. Historical work packages do not reopen the
integrated seams or change architecture authority and the release gate.

## 6. Release gate

Release only after the product path is the proven path.

Required:

- current-HEAD source-bound receipts;
- packaged Windows launch on target hardware;
- current GPU and graphics performance measurements;
- autonomous lifecycle and fresh-process persistence proof;
- live rendered playtest evidence;
- external tester feedback and resolved critical issues;
- explicit release approval.
