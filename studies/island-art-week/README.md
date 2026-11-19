# Island art week – work in progress

Single graphics branch for iterative Qualia island art. Existing terrain pass is PR #412;
the isolated physical V2 and its compatibility check remain under ../island-ridge-v2.
V1 remains the default. No brains, user saves, or terrain authority change here.
No user-PC, native gameplay, or RTX testing during art week.

**Current study: eastern strand.** Actual terrain triangles determine deposit positions
and support every stone, rolled ribbon of wrack, scallop and tuft. The narrow wrack
and upper salt-reed bands occur only on suitable outer shores and break into local
drifts with open stretches. Two shared PBR finishes and spatially bounded patch meshes.
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

**Resume point:** inspect the paired strand renders before making additional assets.
If the material feels scattered like confetti, lengthen the deposits, reduce loose shells,
and cluster kelp folds. Bind the accepted asset as nonpickable, noncolliding, near-only
dressing in the production renderer. Then connect the wet strand to the first meter of
water using the terrain-derived depth rather than an unrelated foam noise texture.

Next week, use a disposable profile and matched native habitat/shore cameras. Test a
walking creature and a rolling toy across the visible contact surfaces. Measure target
RTX cost only when the machine and adapter have been confirmed free; compare the same
camera, resolution, quality settings, save and warm-up. No such test has yet been run.
