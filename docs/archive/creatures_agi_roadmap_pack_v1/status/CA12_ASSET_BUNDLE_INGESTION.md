# CA12 - Asset bundle automation and ingestion

Status: complete.

CA12 adds a central app bundle manifest for the graphical alpha app and a
validation command that discovers and validates config, save, shader, and
placeholder art metadata without generating package artifacts.

## Manifests

- App bundle manifest: `crates/alife_game_app/app_bundle_manifest.json`
- Placeholder art manifest: `crates/alife_game_app/placeholder_art_manifest.json`
- Environment manifest reference: `crates/alife_game_app/environment_manifest.json`

The app bundle manifest covers:

- GPU alpha and P34 runtime configs.
- GPU alpha and P34 asset manifests.
- GPU alpha and P34 portable saves.
- Committed WGSL shader assets under `crates/alife_gpu_backend/shaders/`.
- Tiny text/shape placeholder art metadata for creature, food, hazard, and
  obstacle markers.

## Command

Validate the app bundle:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke
```

Validate a caller-supplied manifest:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
```

## Boundaries

- No large binary assets are committed.
- Shader discovery is validation metadata only; GPU remains optional and
  fallback-capable.
- No Bevy/wgpu/GPU dependencies were added to `alife_core`.
- No release tag or package artifact is produced.
- No full action-authoritative GPU runtime claim is made.

## Focused evidence

Planned CA12 focused commands:

```powershell
cargo test -p alife_game_app ca12 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke
```

Next manifest plan: CAR12 hard-stop review.
