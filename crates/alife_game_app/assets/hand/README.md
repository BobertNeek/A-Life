# Disembodied wizard hand

`wizard-god-hand.blend` is the Blender 5.2 source; `wizard-god-hand.glb` is the
shipping skinned hand. `hand-specification.json` records contact positions for
Hover, Point, Grab, Carry and Release.

Export with `scripts/build_wizard_hand.py -- --no-review` in Blender, then refresh
registrations with `python scripts/register_approved_art.py`. Validate registered
assets through the game app's `validate-production-assets` command.
Physical holding belongs to the world; the renderer projects the resulting pose.
