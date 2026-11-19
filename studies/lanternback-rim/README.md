# Lanternback Rim

[Two matched camera pairs](gallery/Lanternback-Rim.jpg) · [Packed editable workbench](gallery/Lanternback-Rim.blend) · [Four full camera originals](gallery/locked-photographs.zip)

At guardian height the previously uninterrupted brown-green hillside now has broken, cool-grey exposures running down through ochre heath, separated by living ground rather than stripe-like terraces. At the matched 1.04 m eye the diagonal bank gains a legible line of exposed stone, scattered gravel and upright dark-copper seedheads against the distant sea. The close sandy soil between these tufts still lacks convincing grain and litter.

## What the island provided

The eastern high ground descends from 79.75 to 5.99 metres along the surveyed line toward Sunspindle. The [70 actual samples](gallery/slope-section.csv) record the original triangle height and gradient, not a new path or a walkability study. The native shapes, prop positions, sunlight, original normal and textures are held. No summit, cliff, solid, save, game file or simulation changed.

The independent art area spans 121 × 120 m. Eight unequal, broken bedrock exposures are limited by the actual grade. They cross each other and pinch out instead of forming a repeated horizontal line. Within them, 92 outcrop groups split into 237 irregular caps with real shallow lips and 616 embedded apron fragments (20,194 triangles total). The original slopes measured at the 92 selected sites run from 22° to 50° (mean grade 0.62). The cap polygons are clipped on both the grid and its real diagonal before their heights are fitted; their highest design allowance is 9 cm, and on gentler ground it drops to 2.3 cm. These are visual surfaces, not collision. Plant and slab origins use buffered original solid bounds. This is not a native clearance claim. Native rocks are not moved into the shots.

The verges contain 660 instances of nine models (201 wind-heath stools, 114 rose-scale pennons and 345 small cushions), seated on the actual slope. The shared palette is 2,520 triangles, 166,116 triangles instanced over the entire site. Those are asset counts, not engine draw calls or an FPS claim. A world-space chart weights exposed mineral, sheltered turf and olive-copper seams; separate small-scale grain and relief plates are masked by the rock field.

## Reopen

The packed file contains the exact cropped V1 triangles, editable linked heath, both original cameras, the three-dimensional fitted rock and the small-scale material source. The photographs, unlike the neutral workbench crop, include the wider native island and original registered props. Both camera pairs are 840 × 500, Blender 4.3.2 CPU, 24 samples, one thread, with the same settings before/after. This is offline art; no validation CI, native renderer, game, benchmarks, or user machine were used.

The parent is the independent Sunspindle hold `41fb18b5045b0fbcee58e3192d3554539c91c7d2`. [Native input ledger](gallery/native-hold.json) records the actual immutable Git blob IDs used to construct the art scene. To recreate: make an input directory from the island assets at this ref with the separate alife_world island-v1.bin copied beside them; run `python author.py INPUT OUT/art` and `python layout.py INPUT OUT/scene`, then `blender -b -t 1 --python-exit-code 1 -P photograph.py -- INPUT OUT/scene OUT/art ../sunspindle-hollow/gallery OUT/photo pairs`. Pass `desk` instead of `pairs` to save the editable source. Pass absolute input and output paths, and create OUT/photo initially. Blender needs its bundled glTF importer; the outside authoring Python uses NumPy, SciPy and Pillow.
