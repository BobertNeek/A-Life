# Legacy True 2.5D renderer archive

This directory preserves the retired alpha-art and True 2.5D graphical stack as source reference. It is not part of any Cargo target, app bundle, launcher, package, or test gate.

The archived files came from commit `746abd6a2fbc668710ab1513713da85656b0773b`. The snapshot includes the old mixed Bevy shell, playground runtime, sprite and GLB asset packs, stylization shader, generation tools, launchers, package scripts, presentation-only milestone modules, and their focused examples and tests.

The active game uses the production Vulkan voxel frontend in `crates/alife_game_app/src/bevy_shell.rs`, `crates/alife_game_app/src/production_voxel_renderer.rs`, and `crates/alife_game_app/assets/production_voxel_v1`.

Do not restore individual archived files into active targets. If an old idea is useful, port the smallest relevant behavior to the production renderer and keep simulation authority in the existing world and GPU runtime.
