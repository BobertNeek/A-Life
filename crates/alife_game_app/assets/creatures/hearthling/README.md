# Hearthling production model

`hearthling.blend` is the editable refined biped; `hearthling.glb` is the shipping
skinned mesh with idle, walk and sleep clips. Runtime appearance and animation
project organism state. Walk phase follows ground distance, with a 0.96 model-unit
left/right cycle scaled to the creature.

Export with Blender 5.2.1 using `scripts/export_hearthling.py`, then refresh
registrations with `python scripts/register_approved_art.py`. The exporter uses
the retained sculpt, face, eye, fur and runtime optimization scripts.
`scripts/render_hearthling_states.py` reimports the exported model for visual
inspection. Validate registered assets through the game app's
`validate-production-assets` command. Historical revision evidence remains in Git.
