# Island art week – work in progress

Single graphics branch for iterative Qualia island art. Existing terrain pass is PR #412;
the isolated physical V2 and its compatibility check remain under ../island-ridge-v2.
V1 remains the default. No brains, user saves, or terrain authority change here.
No user-PC, native gameplay, or RTX testing during art week.

**Current study: eastern strand.** Actual terrain triangles determine deposit positions
and support every stone, rolled ribbon of wrack, scallop and tuft. The narrow wrack
and upper salt-reed bands occur only on suitable outer shores and break into local
drifts with open stretches. Four shared PBR finishes: wet stone, dry reed, a thin sand film, and a depth-fitted, cellular-lace wash.
A dedicated hosted CPU Blender study locks the light and both cameras, uses real assets,
and records an independent V1/V2 geometry receipt. Water is a stand-in on these plates.
Do not mistake them for screenshots or FPS evidence.

To reproduce the artwork and plates in a checkout with a compatible Blender:
1. Run `python scripts/build_island_strandline.py --out /tmp/strandline.glb`.
2. Run `python scripts/verify_island_strandline.py /tmp/strandline.glb`.
3. Run the two `render_island_geology.py --tidal-review` invocations in
   .github/workflows/island-art-study.yml. Only the second includes the strandline GLB.
4. Review the 86cm shoreline view and the bank view together. Inspect scale, silvery
   wet highlights, animation-free contacts, visible repetition and patch culling.

**Accepted coastal checkpoint:** [eye-height before/after](strand-eye.jpg),
[bank before/after](strand-reach.jpg), [independent geometry receipt](strand-validation.json).
Three passes were compared: loose beach dressing alone, a damp strip with over-broad
foam, then the present broken fine wash. The wet film is 3 mm above the source plane;
solid-looking shingle never projects more than 4.21 cm. The actual form is batched in
50 m tiles inside the existing 100 m chunks, not entities per pebble. The whole
island contains 137,431 decoration triangles; no patch exceeds 6,989. Patches are
nonpickable, do not cast shadows, and are visible only over the exact near terrain.
Neither these counts nor Blender timings are game FPS.

**Resume point: inland ground contact and visual scale.** Inspect the current plant/ground
edges from the 88 cm nursery view and a second camera on the forest approach. Keep the
centre clear. Give the rooted bases small occluded leaf fans and interwoven seedling
patches at their actual anchors, with authored gaps. Avoid duplicating the earlier
nursery understory, painting a general noise carpet, or changing interactive plants.
Judge both silhouettes and colour against the locked original. Next, inspect shadow
hierarchy and near/far continuity at the two asymmetric northern summits. Use a single
weekly branch, run isolated renders remotely, and keep the learner's workspace free.

Next week, use a disposable profile and matched native habitat/shore cameras. Test a
walking creature and a rolling toy across the visible contact surfaces. Measure target
RTX cost only when the machine and adapter have been confirmed free; compare the same
camera, resolution, quality settings, save and warm-up. No such test has yet been run.
