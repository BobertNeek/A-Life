# Disembodied wizard hand

`wizard-god-hand.blend` is the measured Blender 5.2.1 LTS source. It has a
continuous skin, short rounded nails, 16 bones, and five poses: Hover, Point,
Grab, Carry, Release. The palm is 1.35 metres long, 0.99 wide, 0.33 deep; the
middle finger is 1.65 metres long. The total length is 3 metres. Source Z-up
becomes runtime Y-up through glTF export.

Carry uses a thumb/index pinch at the creature's upper back. The hand remains
disembodied. Runtime hover size follows orthographic viewport coverage; carry
uses its measured world scale. The skin is 10500 triangles, shared by all poses.
The approved references and measured guides are inside the source.

Physical holding belongs to the world. The renderer displays the resulting
position. A quarter-second hold grabs; releasing the mouse, losing focus, or
pressing Escape drops onto the last valid terrain support. Selection is a
material highlight. Window-edge scrolling remains available while carrying.
