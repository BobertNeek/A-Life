# alife_game_app

Own the playable application, launch policy, UI, and presentation. Keep Bevy
integration feature-gated and the default build headless. Use the shared neural
runtime and world validation rather than a second simulation in the renderer.

Preserve save/load, acquired learning, founder reconstruction, and archive-before-
retirement ordering. Apply or discard a pending learning transaction only after
its matching world outcome is sealed. Observe outcomes in organism memory even
when learning is rejected. Keep developer training state out of player saves.

Production art uses the skinned Hearthling and Blender landscape props. Update
exported GLBs and manifest digests together; retain useful editable sources.
