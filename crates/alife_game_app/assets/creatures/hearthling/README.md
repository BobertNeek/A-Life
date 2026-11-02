# Hearthling production model

`hearthling.blend` is the editable refined biped; `hearthling.glb` is the shipping
skinned mesh with idle, walk and sleep clips. Runtime appearance and animation
project organism state. Walk phase follows ground distance, with a 1.44 model-unit
left/right cycle scaled to the creature.

Export with Blender 5.2.1 using `scripts/export_hearthling.py`, then refresh
registrations with `python scripts/register_approved_art.py`. The exporter uses
the retained sculpt, face, eye, fur and runtime optimization scripts.
`scripts/render_hearthling_states.py` reimports the exported model for visual
inspection. Validate registered assets through the game app's
`validate-production-assets` command. Historical revision evidence remains in Git.

The selected 09/16/19 pass rounds the existing digits and palm transition,
softens the crown, and bakes finer fur relief into the existing shared atlas.
The cream upper-bib projects less like stacked collars and its lower tips taper
centrally; ear-lock tips are staggered toward the selected 16/19 references.
Topology, vertex colours, skin weights and the shared-material budget are retained.
`refine_hearthling_coat.py` can apply this follow-up once to a pre-pass editable
checkpoint; the full source exporter includes the same pass.
`refine_hearthling_selected.py` applies it reproducibly before the fur bake;
the original source, rig and three clips remain the starting point. Runtime
compact/neutral/broad paw proportions read the saved arm-family choice and reuse
the existing hand bones, with a neutral fallback for unrecognised families.
The re-import review checks every authored frame at compact, neutral and broad
paw scales, then renders the three clips and facial blinks. Set
`HEARTHLING_REVIEW_CPU=1` for CPU Cycles review; these previews are asset checks,
not native gameplay or frame-time evidence.

## Chubby raccoon-shaped extremities (2026-10-28)

The existing Hearthling now has five rounded, varied-length digits per hand,
a short fifth side digit and cushioned palm, plus five distinct toes and a longer,
flatter sole. This changes the extremities, not the selected creature identity.
Palm pads and finger curl share the same side; fifth-digit placement is mirrored
with the correct chirality. The wrist is welded into the forearm and the ankle
into the leg, rather than covering either joint with overlapping mitten shapes.

The native source retains the original 34-joint rig and three clips. Individual
fingers do not add bones or independently animate; they reuse each existing hand
bone and the compact/neutral/broad presentation scale. The six materials, one
packed normal atlas and its two texture registrations are unchanged. New smooth
surfaces use a checked, flat corner of that atlas. The original sculpt and all
prior refinement stages remain the reproducible source chain.

`refine_hearthling_extremities.py` applies this pass once to a pre-extremity
checkpoint (for example the editable asset at commit `240f434c`). Reapplication
is rejected before writing. To keep a review export separate:

```sh
blender --background --python-exit-code 1 --python scripts/refine_hearthling_extremities.py -- --source /path/to/pre-pass/hearthling.blend --output-dir /path/to/review
```

The full `export_hearthling.py` pipeline includes the same pass after the fur
bake. Bridge tessellation is explicitly nondegenerate, and the entry point checks
3D and UV area for every new hand/foot triangle before saving. The re-import
review covers 1,305 clip/profile samples, including quarter-frame sleep poses,
plus grounding and eye occlusion; CPU Cycles previews are not a native
gameplay/FPS benchmark.

| Shipping cost | Before | This pass |
| --- | ---: | ---: |
| Triangles | 32,694 | 32,718 |
| Exported vertices | 33,027 | 33,952 |
| GLB bytes | 3,067,564 | 3,123,208 |
| Meshes / material primitives | 1 / 6 | 1 / 6 |

The increase is 24 triangles, 925 exported vertices and 55,644 bytes, without new
material, texture, joint or clip cost. Native frame-time impact remains unmeasured.

### Sleep hand clearance

An animated audit found the existing left hand crossing the folded knee/thigh
in the seated sleep pose, including before the raccoon-shaped extremity pass.
The sleep clip now lifts the existing left upper-arm joint by 0.18 radians;
the approved hand/foot geometry, rig, materials and all idle/walk channel values
are unchanged. The shipping GLB is 3,122,928 bytes with the same geometry cost.

`refine_hearthling_sleep_pose.py --source /path/to/pre-clearance/hearthling.blend
--output-dir /path/to/review` applies this once to the accepted native checkpoint;
the full exporter includes the same correction. The re-import review now also
rejects hand-versus-thigh/shin surface intersections on both sides at 289
quarter-frame samples across the 73-frame sleep clip, at compact, neutral and
broad paw scales. This discrete sampling is not a continuous-time guarantee.
This is asset-level pose validation, not native locomotion, contact physics
or FPS qualification.
