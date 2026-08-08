# Roadmap

The roadmap is ordered by causal dependency. A later phase does not compensate for an earlier missing authority link.

## 1. Live GPU-to-voxel projection

Build a read-only presentation adapter keyed by stable organism and world entity IDs.

Exit gate:

- a real GPU-selected action changes the authoritative world;
- the matching voxel entity receives the resulting transform or visibility change;
- renderer code cannot write world truth;
- one receipt binds input, GPU selection, world outcome, and render update;
- before/after rendered evidence shows the same organism.

## 2. Autonomous production lifecycle

Connect birth, ageing, reproduction, death, and lineage archive transactions to the active GPU schedule.

Exit gate:

- a multi-tick production run creates and retires organisms without a test harness calling the transaction directly;
- birth archive precedes GPU insertion;
- final seal and life archive precede GPU retirement and despawn;
- stable IDs, GPU slots, lineage, and visible entities agree after each transition;
- save/restore across the lifecycle preserves authority.

## 3. Truthful player controls

Bind pause, speed, load, save, selection, follow, and new-world commands to the same live runtime and projection.

Exit gate:

- pause stops simulation and presentation at a sealed boundary;
- load atomically replaces runtime and scene or fails without partial state;
- save/load succeeds across a fresh process;
- controls report unavailable neural hardware honestly;
- disabled or diagnostic-only features are visibly labelled.

## 4. Repair EI1

Treat the retained `Blocked` corpus as diagnosis, not promotion.

Exit gate:

- the exact source, tree, adapter, backend, config, and corpus are bound;
- intact scores are positive;
- treatment is not worse than control in every required cell;
- every minimum control margin passes;
- required plateau windows are complete and valid;
- the causal zstd sidecar streams successfully and matches every promoted receipt;
- focused gates contain no EI1 ignores.

If measured ability still fails, retain `Blocked` and improve the training design rather than relabelling the evidence.

## 5. Prove the player loop

Demonstrate the product, not only its components.

Exit gate:

- player input reaches the live world;
- a creature perceives, selects, acts, and learns through the GPU path;
- the world changes and the renderer shows that change;
- speech, selection, inspection, save, load, sleep, birth, and death use the same identities;
- fresh-process and real-input checks produce causal and rendered evidence.

## 6. Scale by measurement

Keep brain class and population capacity as separate decisions.

Exit gate for each production class and population profile:

- complete causal, learning, sleep, restore, memory, topology, and rollback evidence;
- measured p95 tick and VRAM use on the named adapter/backend;
- populations 1, 10, 50, 100, 250, and 500 measured for both sensor profiles where required;
- admission, throttling, save migration, soak, and replay gates pass;
- N4096 remains research-only until its own equivalence and rollback gate passes.

## 7. Release gate

Release only after the product path is the proven path.

Required:

- current-HEAD source-bound receipts;
- packaged Windows launch on target hardware;
- current GPU and graphics performance measurements;
- autonomous lifecycle and fresh-process persistence proof;
- live rendered playtest evidence;
- external tester feedback and resolved critical issues;
- explicit release approval.
