# Roadmap

Updated 20 September 2026. The immediate priority is the persistent creature game
described in section 1.1 of the controlling v2.0 architecture: care, useful
learning, and save/load first; breeding and practical breed promotion next.

The [core game loop delivery plan](core-game-loop-plan-2026-09-20.md) contains the
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

Follow the [approved promotion scope](reviews/2026-09-11-breed-promotion-scope.md).
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
