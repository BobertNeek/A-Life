# Roadmap

Prioritize the persistent creature game: care, useful learning, and save/load;
then a small community, breeding, and practical breed promotion. The
[organism and gameplay design](architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md)
contains the longer-term ambitions. Research mechanisms are selected when they
help that game or answer a specific learning question.

## A working care loop

Use the actual production terrain, a fresh save, and one creature to connect
reachable food, eating, rest, recovery, readable needs, and direct care tools.
Observe a real GPU-selected action, its world consequence, and the matching
visible creature. Fix the observed break and preserve existing saves.

A useful lesson should change later independent behavior. Natural sleep and a
fresh-process restore should retain the same individual and its acquired skill.
Pause, speed, save, load, selection, follow, and new-world controls should continue
to operate on the same runtime and presentation state. Failed load must leave a
usable prior state; unavailable hardware should be explained clearly.

## Community and generational play

Extend that loop to a small community with useful objects, sustainable resources,
readable social behavior, reproduction, inherited variation, development, death,
and lasting lineage records. Offspring need fresh identities and meaningful
inherited differences. Birth archive precedes GPU admission; final life archive
precedes retirement. Restoring a save should restore that same family.

Practical named breed promotion should retain a useful skill in a fresh
descendant, clear personal records, and leave the source individual unchanged.
One selected individual is sufficient; cohort research is optional. Ordinary
mating does not copy an acquired adult mind. The
[historical promotion decision](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/reviews/2026-09-11-breed-promotion-scope.md)
explains that distinction.

## Learning questions

Investigate a concrete learning problem with an appropriate comparison and later
unprompted behavior. Preserve acquired state and report failed or missing
observations. Historical EI campaigns remain in [Evidence](EVIDENCE.md); their
score matrices and review eligibility do not gate a gameplay fix.

## Performance: fifty individuals at 20 FPS

The user's target is i7-3770K / 32 GB / RTX 3050 8 GB at 1080p, with the existing
20 Hz simulation/cognition schedule, useful learning, individual memory, and exact
continuity. **Fifty creatures at 20 FPS remains unmeasured for the current source.**

Measure the supported play session on available hardware. Record source,
configuration, population, brain/sensor profiles, useful neural work, progressed
ticks, frame/tick latency, CPU/GPU memory, and the stage responsible for a real
bottleneck. Fresh and populated memory, crowded scenes, and sleep can affect the
result; choose comparisons relevant to the question rather than a universal
population/profile matrix.

Existing work has removed discarded preparation copies and bounded CPU
preparation. Further reuse, world-query, allocation, transfer, or presentation
changes should follow measurements. Preserve grounded candidates, private state,
GPU action selection, cognition cadence, and ordinary save/load. Hardware work
needs an available resource window; this roadmap does not schedule work on the
user's PC.

The [historical optimization plan](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/performance/20261001-fifty-creature-optimization-plan.md)
preserves earlier arithmetic and alternatives. Completed implementation and
review logs remain in Git history.

## A releasable game

Exercise the packaged Windows application on supported hardware: care and later
learning, a complete family lifecycle, truthful controls, and retained individual
state across restart. Measure practical graphics/cognition performance and
resolve observed critical defects. Use player feedback to improve the experience
before the user's release decision. The checks should support those claims
without adding a separate reporting framework or research campaign to each fix.
