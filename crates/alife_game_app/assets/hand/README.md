# Disembodied wizard hand

`wizard-god-hand.blend` is the measured Blender 5.2 source. It has a
continuous skin, short rounded nails, 16 bones, and five poses: Hover, Point,
Grab, Carry, Release. The palm is 1.35 metres long, 0.99 wide, 0.33 deep; the
middle finger is 1.15 metres long. The total length is approximately 2.5 metres. Source Z-up
becomes runtime Y-up through glTF export.

Carry uses a thumb/index pinch at the creature's upper back. The hand remains
disembodied. Runtime hover size follows orthographic viewport coverage; carry
uses its measured world scale. The skin is 10500 triangles, shared by all poses.
The original references remain packed inside the source. The approved October
3 revision has fuller fingers, relaxed open poses and a curved back with softer
knuckles. The right-hand geometry, rig and winding contain the reflection;
the exported root has positive unit scale. In runtime camera coordinates the
dorsal surface faces the player, fingers point up and the thumb is on the left.
All five poses share that camera-relative orientation.

`hand-specification.json` records each pose's measured glTF contact. Hover,
Point and Release use the index endpoint; Grab and Carry use the thumb/index
pinch center. The renderer's matching contacts are verified by
`scripts/check_wizard_hand.py`. Background export with
`scripts/build_wizard_hand.py -- --no-review` performs no review render.
Refresh only the hand manifest entry with `scripts/register_wizard_hand.py`,
then run the focused hand and existing island art checks.

Physical holding belongs to the world. The renderer displays the resulting
position. A quarter-second hold grabs; releasing the mouse, losing focus, or
pressing Escape drops onto the last valid terrain support. Selection is a
material highlight. Window-edge scrolling remains available while carrying.
