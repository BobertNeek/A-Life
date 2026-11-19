# Fern hollow, island studio

This is a close woodland dressing asset, not a new or interactive village object.
It is built from existing pines and broadleaves in `props.json`; it writes no
placements, navigation rules, collision bodies, state components or save data.

- `scripts/build_island_copse_floor.py` selects protected root-contact patches.
  Selected origins need a moderate real slope, nearby trees and an unchanged
  circle of V1/V2 height samples. Original broadleaves already have root flares.
- `scripts/island_copse_botany.py` grows aperture-separated ground ferns, sorrel,
  moss, needles and exposed root feet in unequal clusters. The woody flutes
  enter the original pine bole; their clearance falls sharply beyond its foot
  into a few centimetres of ground dressing. The tall part is not a detached
  obstacle that a creature could walk through.
- `scripts/island_copse_mulch.py` paints loose woody litter and clips each
  scuffed fall against the exact underlying near-LOD triangles. No flat decal
  is suspended on the downhill side, and the same physics mesh remains in use.
- `scripts/island_spruce_bark.py` unwraps the stock bark without changing a
  root or trunk polygon. Low vertical sides no longer borrow the
  downward end-cap normal that gave the original trees a black saw-cut. The stem and the outer flutes use related scale and
  complexion. The selected existing spruce canopy continues to use the other
  half of its original cutout atlas. Both materials have albedo, normal and
  AO/roughness maps; the physical lighting is still the game's responsibility.

For art-only reproduction, the feature branch's Blender studio-board subject is
`forest-floor` with `sculpted_bark: true`, `paired: true`, and
`angles: ["sunlit-margin", "copse-eye"]`. Use the original 1.04 m foreground at
x=253.40,z=-207.05 as the first locked slope; the other looks into the sunlit west-facing woodland
margin at original planted pine 952 from x=-149.85,z=6.18. The shaded
older pine 2/934 setup also survives as the optional `fern-hollow` framing. An artist's 1.04 m perspective
view is not the game's orthographic camera. Neither GPU cost nor runtime
streaming/interaction has been checked during the owner's studio-only week.

**At the next booked test window, not before:** compare the same clean V1
profile against `5d6181e662ddd9e6cf8daf16f4c9d39a5ef1fe9c`. Check a real
creature and the rolling toy near both pines and on the clear path between them.
Look for stem/flare seams, floating feet, cutout shimmer, glints or the terrain
bleeding through incorrectly. The forest-floor patches are optional, hidden
beyond 65 m focus distance and a 60 m view span; they have no picking or shadow
casting. Check close-to-wide transition in motion and record actual RTX 3050
frame time separately. Preserve the previous pine pack during any rollback.

The historical `build_island_boughs.py` no-argument material variant and its
cap/UV equality proof describe the earlier white bark swatch. They are **not**
validation of the repaired side shading or the current rolled atlas. Reproduce
this specific candidate with the `sculpted_bark=True` artist export. Next
week, retain its physical position/indexed-triangle/anchor comparisons, permit
UV seam splits as vertex storage only, check the untouched bottom-cap positions
and verify that vertical bark normals no longer point into the ground. Use the
existing native loader checks on the current installed release, not a fresh
export of the old plain-bark default.
