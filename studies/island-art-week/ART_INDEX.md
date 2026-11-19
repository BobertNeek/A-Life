# Which island artwork is this?

All photographs here are Blender artist views. None demonstrates gameplay, a frame rate, picking, collision contact or save restore. The weekly art branch is not the shipping branch. Use [the disposable next-week native recipe](NEXT_WEEK.md) before any promotion; do not use an active player profile or a learner checkout.

| Subject | What is actually selected | Physical surface |
|---|---|---|
| [The original terrain pass, PR #412](https://github.com/BobertNeek/A-Life/pull/412) | Material, near relief and far representation. | V1 stays the default. |
| [Optional ridge alternative](../island-ridge-v2/README.md) | Separately versioned highland shape; this is the alternative used in the wide coastal composition. | Own render **and** collision surface. It does not change V1 old saves. |
| [Cove composition](habitat-cove.jpg) and [the original boulder-eye comparison](bough-creature.jpg) | Broader windbreaks, grounded stones, warm sun and a cooler sea fill. | V2 named in the composition; the technical eye/bank controls remain V1. |
| [Recent native woodland](WOODLAND_SOURCE.md) and [mature alder](ALDER.md) | Wind-shaped spruce, grounded roots and duff, divided fern, real fork-grown mature alder, and their embedded edits on the weekly art branch. | Original real ecology anchors and frozen forest proxies. |
| [Wild lower valley](MEADOW.md) | `a50dca41`: native Grass/Flowers and two local soil/plant tiles beside original 3406, around (75,301). Not a cultivated village bed. | Display-only around the existing V1 / V2-identical physical triangles. |
| [Young saltwood](YOUNG_SALTWOOD.md) | `10e78faf`: all 164 original saplings. Former solid green balls are replaced by visibly separate leaves and boughs around the retained wood. | Original roots, forks, placements and save IDs; no new colliders. |
| [Southern V1 bank](INLET.md) and [close-surface diagnosis](INLET_UNDERPAINT.md) | Existing rooted local material and a separate documented feathering change/study. Use the individual project's *selected* receipt, not an unreviewed artist snapshot. | Original V1 bank; no modified height or water level. |
| [Southern editable channel proposal](../island-inlet-proposal) | `507a2cbd`: three camera comparisons and a packed, artist-editable `.blend`, not a playable island variant. | **No runtime binding at all**. |

For the first native review, use the game’s own orthographic camera and a paired saved pose. The 1.04 m DCC valley eye is a ground-art magnifier, not a camera promised to ship. Record actual GPU, fps and frame-time tails, allocation, alpha-leaf/shadow load on forest zooms, visibility gates and fallback assets. Terrain and vegetation edits should never affect active learning or require opening an existing world.

[Planted cliff contact proposal](../island-scarp-contact): separate editable display study, not a new physical ridge or an installed game change.
