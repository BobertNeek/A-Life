# Qualia: the Lantern Garden

## Direction and limits

The reference is the relationship with a creature in *Black & White* and the tactile nursery of *Creatures 3 / Docking Station*, in a cozy, soft science-fiction world. This is a production slice, not a claim of AAA polish. Preserve the selected, cute mammalian Hearthling, its rig and its body language. Aim for craft and clarity: a touched system should leave less duplication behind, not a new overlapping convention.

The ordinary game teaches with silhouettes, motion, light and repeated pictograms. Visible words belong only to actual creature speech bubbles and self-explanatory, single-word main-menu buttons. Screen-reader descriptions stay intact. Diagnostics and the textual lineage laboratory require an explicitly developer-enabled launch; they are not a substitute for a player-facing family tree.

Do not use this branch to adjust biochemistry, cognition, neural weights, training, or the learning policy. Do not simulate success because the Hand asked for something. Unknown evidence stays unknown.

## What the player can do

* Notice: meet the Hearthling under the amber focus lantern. The selected individual's crest is stable across sessions; the quantity chips distinguish absent evidence from a measured zero.
* Approach: point the Hand and right-click once to stroke the individual under it. Positive and negative interactions remain separate, deliberate actions. Holding the button never queues repeated pets.
* Offer: aim at ground to place the orchard fruit, or select a creature to offer fruit just ahead of its gaze. An explicitly blocked location is not silently changed. A creature-directed offer seeks a nearby, walkable alternative. The real food and toy belong to the world, not the renderer.
* Play: place the orbital rattle near a creature and leave the outcome up to the creature. A dropped prop, a requested touch and consumed care are different events. Consumed care has an action-shaped glyph and bodily response; it never generates pretend dialogue.
* Communicate: tap grounded word pictograms, select an addressee or broadcast, and send through the existing speech channel. Only actual utterances anchor to the live creature. Typed input and assistive names remain available.
* Return: familiarity records an individual's history of consumed player care. It decays over simulated time, persists through save/load and colors the presentation of an existing rest pose. It is **not** a new neural input or hormone.

## Integration

Branch: `qualia/lantern-garden-20261118`, based on `52badd43`. Compared the actual combined Argon branch `argon/brain-and-game-improvements-20261110` (`eacb100d`), including its merge of Muse `bob/soul@f08b7755`, against current main. Kept main's working carry/camera, load-identity and monotonic-care-receipt fixes; brought over Muse's presentation/save slice, not either branch's research paths. In particular, an old or missing care receipt cannot stand in for an observed reaction.

Art source, release meshes and a Blender material study live in [assets/care](../crates/alife_game_app/assets/care). Rebuild the native scene and glTF meshes with `blender --background --python scripts/build_care_keepsakes.py`. A material study is not a screenshot of gameplay. In-game colors derive from the world object's appearance; visual assets never own selection, placement or effects.

## Verification and remaining work

```text
cargo fmt --all -- --check
cargo test --locked -p alife_world --test canonical_new_game player_familiarity
cargo test --locked -p alife_game_app --features production-voxel-frontend --lib production_voxel_renderer
cargo test --locked -p alife_game_app --features production-voxel-frontend --lib production_conversation_lineage_ui
cargo run --locked -p alife_game_app --features production-voxel-frontend --bin alife_game_app -- validate-production-assets
```

Use the optional `lantern_garden_playtest` example for a real, GPU-required 960×540 smoke playthrough. It starts a **new** seed under a fresh artifact directory, captures frames of a normal menu dismissal, offering, petting, play and conversation, and never injects keyboard input into another application. On Windows MSVC the production loader needs a 16 MiB main-thread stack: build the example with `cargo rustc --locked -p alife_game_app --features production-voxel-frontend --example lantern_garden_playtest -- -C link-arg=/STACK:16777216`, then launch the example binary. Check the screenshot before claiming an interaction was completed; queued care and an observed response are distinct.

A human first-time-player study, an accessibility-device pass, pictorial family-tree play and long-session observation are still needed. They cannot be inferred from a screenshot or CPU fixture.
