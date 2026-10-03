# Player hand repair and visual preview — 2026-10-03

Base: main `d2e4a157ec8b14fc21445d55d560ab36c4551d49`.
Local worktree `/workspace/alife-hand-grab`; interaction repair branch
`phase3-player-hand-interaction`. No main merge, training, GPU dispatch or
native game run occurred. Visual asset changes remain preview-only.

## Local repair

Terrain hover previously populated `hand.pressed`, immediately selecting Grab
for the quarter-second pickup delay. The eventual hold rejected terrain, so the
visible gesture contradicted the physical action. Pending grabs now require a
Creature/Resource stable ID and the same world-owned object eligibility used by
`begin_player_hold`. The original eligibility checks were factored without
changing their order or errors. Terrain selection/navigation and creature
follow remain separate from pending pickup.

Supported: live available creatures, food, movable balls and tokens. Rejected:
bare terrain, misses/UI-captured pointer samples, missing IDs, unavailable
objects, fixed activity toys, obstacles and hazards. Production picking already
enumerates the four supported object kinds; this is not a creature-only fix.
Pickup, move and release still mutate canonical world state, use its terrain
support policy, clear object velocity and persist the hold. No visual-only hold
or new movement legality was added. A rejected move retains the last valid
support; release returns the object there. The 0.25-second threshold and
mouse-release/focus-loss/Escape cancellation are unchanged.

Source: [pending gesture](../../crates/alife_game_app/src/production_voxel_renderer/god_hand.rs),
[world hold law and tests](../../crates/alife_world/src/headless/player_hand.rs),
[production picking](../../crates/alife_game_app/src/production_voxel_renderer.rs).

## Clarified visual target and verified geometry

Cassidy requested the player's right hand, viewed from behind, with fingers
pointing toward the top of the screen/away from the player. The existing GLB
is a left hand in that view: fingers are local +Y, nails are on local -Z, and
the thumb is on local -X. In Bevy camera coordinates (+X screen right, +Y up,
+Z toward viewer), a dorsal fingers-up rotation `Ry(pi)` puts that thumb on
screen right. A proper rotation cannot change anatomical handedness.

The current non-carry orientation is fixed in world coordinates; carrying
switches to a camera-relative orientation. Correct the asset's handedness, then
use `camera.rotation * Ry(pi)` consistently across all five poses. With the
corrected asset, fingers remain camera +Y, dorsal surface camera +Z, and thumb
camera -X. Normalize the final source geometry/rig/animation and winding;
the preview's negative rig scale is not a final shipping repair.

The current non-carry contact anchor is `(-0.345, 2.8, 0.05)` even during Grab;
the intended source pinch is near `(-0.32, 1.46, 0.59)`. Inspecting the actual
main GLB's first keyframes found skeletal pinch centers of approximately
`(-0.265, 1.376, 0.763)` for Grab and `(-0.271, 1.398, 0.738)` for Carry.
Index/thumb skeletal endpoint gaps are about 0.449 m and 0.388 m, respectively,
rather than the source IK targets' 0.12 m separation. These are bone endpoint
estimates, not mesh collision measurements. Fix/bake the exported poses first,
then derive per-pose contact anchors from the actual exported asset. Do not
copy the old source target constants into the new renderer and assume a match.
During pending Grab, place its contact at the selected canonical object's
contact, using the same object/creature contact rule as Carry. Preserve world
pickup height, movement and release locations.

Source: [hand authoring](../../scripts/build_wizard_hand.py),
[runtime pose/anchors](../../crates/alife_game_app/src/production_voxel_renderer/god_hand.rs),
[asset specification](../../crates/alife_game_app/assets/hand/README.md).

## Concrete preview

Separate CPU source previews are available at
`/workspace/alife-hand-preview/index.html` and
`/workspace/alife-hand-preview/pose-comparison.png`; individual Hover, Point,
Grab, Carry and Release images live under its `current/` and `candidate/`
directories. Production assets are unchanged.

The proposal retains palm dimensions, skin/nail materials and authoring style,
corrects handedness, and changes finger lengths from Index/Middle/Ring/Little
1.55/1.65/1.58/1.25 m to 1.10/1.15/1.10/0.90 m. Palm length remains 1.35 m;
thumb MCP-to-tip remains 0.86 m. This is a proposed proportion choice, not an
anthropometric claim or recovered original approval.

The first preview used CPU Cycles, one thread and eight samples in Blender
4.3.2; the revised preview uses 32 samples. That version cannot open the
compressed checked-in Blender source. Its decompressed header independently
confirms Blender 5.2 (`BLENDER17-01v0502`). Preview generation reuses the checked-in hand authoring
script in a separate namespace and output directory, without modifying it.
The old exporter emits float vertex colors, so previews skip the production
compactor and disable unavailable denoising. Preview GLBs/Blender files are
not production-qualified replacements. Original packed references were not
opened; a terrain review image is not evidence of hand approval.

Exported pose measurements and the inspection script are at
`/workspace/alife-hand-preview/exported-pose-contacts.json` and
`/workspace/alife-hand-preview/inspect_glb_contacts.py`.

Cassidy's subsequent feedback requested fuller fingers, relaxed open poses
and a curved back with softer knuckles. The revised preview retains the
shortened lengths, increases finger radii, bends each open finger differently,
adds gentle dorsal curvature, and reduces the separate knuckle crowns. It is
available in [Library](https://chatgpt.com/api/library/files/libfile_191bd63adde48191a08a45f44ef8641e/download),
version 1, and under `/workspace/alife-hand-preview/revised/` locally. The
comparison places the previous shorter-finger proposal above the revised
proposal. This is a visual review artifact, not a published production asset.

The revised source preview also authors each pose with no active action, then
bakes location, rotation and scale after IK. Its exported Grab and Carry
skeletal endpoints match the intended targets: index `(0.26, 1.46, 0.59)`,
thumb `(0.380039, 1.45994, 0.58994)`, center
`(0.320019, 1.45997, 0.58997)`, gap 0.120039 m. Receipt:
`/workspace/alife-hand-preview/revised-exported-pose-contacts.json`. This fixes
the preview's exported pose mismatch; final production export still needs
independent measurement.

Production export is blocked: only Blender 4.3.2 is installed. The official
Blender 5.2.1 Linux download returned HTTP 403 at the CONNECT proxy, including
an explicit network-permission attempt. No automatic approval rejection was
reported. A compatible Blender 5.2 exporter must be available before replacing
the paired source/export/specification/manifest. The preview's float-color
export must not bypass production compaction or asset validation.

## Verification and remaining work

Three focused world tests passed: supported creature/object pickup, movement,
hold persistence and release; fixed/unavailable object rejection; and the
existing terrain movement/release regression. Renderer tests cover the actual
production eligibility helper and pointer samples rather than running a GPU
session. Their build initially lacked libudev development metadata. The
installed Debian libudev1 runtime is 257.13; a workspace-local linker symlink
and pkg-config description supply that genuine library without system changes.
All three renderer tests passed, including the existing zoom/carry-scale check.
Log: `/workspace/alife-hand-preview/hand-tests.log`.

R2: a separate SOL 6.1 agent reviewed geometry and the local code diff read-only
and found no blocking source issue. It noted that renderer helper tests do not
execute the full input/animation system. Its suggestion to also release the
restored world was added to the world regression test.

Before final visual integration: review the proposed silhouette; author a
normalized right-hand asset and correctly baked five poses; refresh only the
hand's source/export/specification/manifest entries; implement the common
camera-relative rotation and measured per-pose contacts together with that
asset. Tests should cover several camera yaw/pitch/roll combinations and near/
overview zoom, asserting camera-space handedness/up direction and
`root_translation + rotation * (anchor * scale) == world_contact`. Verify each
pose's exported anchor, valid creature/object pickup, terrain/UI misses,
threshold and cancellation, and canonical move/release after restoration.
Then inspect actual native Hover/Point/Grab/Carry/Release at near and overview
zoom. CPU asset images and algebra tests cannot establish that visual result.

No orientation, contact-anchor or production asset change has been applied
yet; those must be integrated with the corrected asset, not shipped against
the existing left-hand model.
