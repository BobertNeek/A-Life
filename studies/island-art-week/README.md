# Island art week

Single working branch for Qualia's coastal habitat. The original terrain
material and geometry pass is PR #412. Its separate [physical ridge
study](../island-ridge-v2/README.md) changes both render and collision for
opt-in testing, not the default map or current saves. No native gameplay or
user-PC performance test has been run during this art week.

## From the dry bank to the windbreak

Water sorts the old constant-height pebble string into grouped anchor stones
and smaller satellites. Eleven reed patches and the wet wrack occupy sheltered
outer shores. The next layer grows only on dry, sufficiently gentle banks with
a real exposed shelf toward water: 82 separate wind-shaped stands, 332 rooted
sprays/clumps. They add olive and silver folded leaves, dry seed combs and
actual short shadows, not glowing point sprites. Existing trees and solid
obstacles keep their original placements, collision and pick behaviour.

The near shore uses five shared finishes. 240 patches on the *whole* island
contain 225,408 triangles; the heaviest is 8,604. The approved eye-height
area selects 28 before frustum culling, 35,058 triangles versus 23,855
previously. The bank selects 45,666 versus 26,497. These are static geometry
counts, not GPU timing. The wet sand is 3 mm off the exact surface, shingle
at most 4.21 cm high, folded plants below 1.10 m. The wash lies 9 mm above
water, not above the submerged floor. Each decoration vertex sits above
identical V1 and opt-in V2 underlying terrain. [Independent vertex and file
receipt](coast-validation.json).

The warm sun and cool fill are an island-only profile. Linear fog begins
beyond the camera's ground focus: zooming the orthographic camera out no
longer puts the habitat itself under a fully developed green veil. Leaving
the island restores the original camera and sun state. Rebuilding an island
scene acquires its replacement sun. The hosted island CPU/ECS suite passed
32 tests including restore/rebuild and camera replay, at `255d80a` (run
44440093415); this is not a GPU or gameplay gate.

## Art evidence and reproduction

[Composed coast A/B](cove-showcase.jpg) · [neutral 86 cm control](coast-eye.jpg) · [neutral bank control](coast-bank.jpg).

The shore-to-massif composition uses the real imported terrain, existing prop
LODs, near-only strandline patches, and the [native water inputs](water-inputs/README.md).
Both sides use the exact same opt-in V2 physical surface, byte-compared to the frozen compatibility fixture. The diagnostic controls use default V1. It is rendered in CPU Blender 4.3.2 (hosted run 44442858406). The same lens is used on both sides; daylight
and atmosphere intentionally change on the revised side. The two established
shore controls hold camera *and light* fixed and still use a neutral sea plane.

Reproduce the matched plates with [.github/workflows/island-art-study.yml](../../.github/workflows/island-art-study.yml).
The earlier beach baseline is pinned to `810e4f872680dc569affa6c8d2e3c31f79caa86c`;
the workflow verifies its SHA-256 before comparison. It verifies the new
geometry separately first with `scripts/verify_island_strandline.py`.

**Prior checkpoint:** [eye-height before/after](strand-eye.jpg),
[bank before/after](strand-reach.jpg), [earlier geometry](strand-validation.json).
This was the first wet film and broken lace pass, before the windbreaks.

**Art verdict and next iteration.** The first air study obscured the mountain;
this one holds its profile and leaves the close plants readable. The near lawn
still reads like coloured gravel and the far trees like isolated pins. Next
give turf, exposed stone and sand *different* close-scale relief instead of
stamping the same ground tile everywhere, then group plant contact/litter by
real rooted habitat. Keep the existing collision and test at the same camera.
A more saturated sky alone would not solve that floor.

**Next week:** [disposable profile, same native camera, contacts and timing](NEXT_WEEK.md).
No established save or guarded learner belongs in a review checkout.
