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
`refine_hearthling_selected.py` applies it reproducibly before the fur bake;
the original source, rig and three clips remain the starting point. Runtime
compact/neutral/broad paw proportions read the saved arm-family choice and reuse
the existing hand bones, with a neutral fallback for unrecognised families.
The re-import review checks every authored frame at compact, neutral and broad
paw scales, then renders the three clips and facial blinks. Set
`HEARTHLING_REVIEW_CPU=1` for CPU Cycles review; these previews are asset checks,
not native gameplay or frame-time evidence.
