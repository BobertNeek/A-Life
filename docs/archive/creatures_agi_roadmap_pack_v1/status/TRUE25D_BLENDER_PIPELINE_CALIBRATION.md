# True 2.5D Blender Pipeline Calibration

## Status

- Branch: `codex/true25d-blender-pipeline-calibration`
- Scope: focused follow-up to the active true 2.5D production presentation goal.
- Roadmap position: post-CA44A visual pipeline hardening; no CA45 started.

## Completed

- Added a Blender normalization tool:
  - `tools/blender/normalize_true25d_assets.py`
  - imports every active true 2.5D asset from the manifest;
  - bakes transforms into mesh geometry;
  - anchors each asset at base-center;
  - scales the largest dimension to `1.0` world unit;
  - applies decimation only if an asset exceeds the configured threshold;
  - exports self-contained `.glb` assets.
- Added a Windows wrapper:
  - `scripts/normalize_true25d_gltf_assets.ps1`
  - discovers the local Blender install when it is not on PATH;
  - writes only untracked receipts under `target/artifacts/`.
- Hardened the seed generator:
  - `scripts/generate_true_25d_alpha_assets.py`
  - now emits seed `.gltf` files under `target/artifacts/true25d_seed_gltf/` by default;
  - refuses to overwrite the active committed asset pack unless `--overwrite-active` is passed;
  - keeps direct seed generation separate from the validated Blender-normalized `.glb` product lane.
- Ran Blender locally:
  - executable: `C:\Program Files\Blender Foundation\Blender 5.1\blender.exe`
  - version: Blender 5.1.0
  - normalized 15 true 2.5D assets.
- Replaced the active unnormalized `.gltf` lane with normalized `.glb` assets in:
  - `crates/alife_game_app/assets/true_25d_alpha_v1/`
  - `crates/alife_game_app/assets/true_25d_alpha_v1/true_25d_manifest.json`
- Updated the Bevy Player View loader to use the normalized `.glb` scene files.
- Tightened manifest validation so each required asset must prove:
  - `blender_normalized=true`;
  - `origin_anchor=base-center`;
  - `transform_applied=true`;
  - `max_dimension_units <= 1.001`;
  - triangle count is positive and within the configured decimation threshold;
  - index count matches the recorded triangle count;
  - files stay under the existing size cap.

## Asset Results

- Assets normalized: 15.
- Decimation threshold: 512 triangles.
- Largest normalized asset: `terrain_resource_grove.glb`, 12,756 bytes.
- Largest triangle count: `selection_ring.glb`, 288 triangles.
- Decimation applied: no; all current source assets were already under the threshold.
- Old active `.gltf` files were removed from the committed asset pack after the `.glb` loader path was wired.

## Focused Evidence

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/normalize_true25d_gltf_assets.ps1 -CheckOnly
```

Result: PASS. Blender 5.1.0 discovered at the local Program Files install path.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/normalize_true25d_gltf_assets.ps1
```

Result: PASS. Normalized 15 assets; max triangles 288; max file bytes 12,756.

```powershell
python scripts/generate_true_25d_alpha_assets.py --output-root crates/alife_game_app/assets/true_25d_alpha_v1
```

Result: PASS as an expected refusal. The script refused to overwrite active committed product assets without `--overwrite-active`.

```powershell
python scripts/generate_true_25d_alpha_assets.py
```

Result: PASS. Seed glTF output was written under `target/artifacts/true25d_seed_gltf/`; no seed artifacts are tracked.

```powershell
cargo test -p alife_game_app true_25d_assets -- --nocapture
```

Result: PASS, 3 passed.

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
```

Result: PASS, 5 passed.

```powershell
cargo run -p alife_game_app --bin alife_game_app -- production-asset-pipeline-smoke
```

Result: PASS. The existing production asset pipeline still detects Blender as locally available.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS after warm build. GPU preflight selected `GpuPlastic` on `NVIDIA GeForce RTX 3050 api=Vulkan driver=581.80`; the normalized GLB Player View smoke exited cleanly.

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. Forced fallback selected `CpuReference` with `HardwareUnavailable` and kept the smoke bounded.

## Full Validation

- `cargo fmt --all -- --check`: PASS after applying formatting.
- `cargo check --workspace --all-targets`: PASS.
- `cargo test --workspace --all-targets`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`: PASS.
- `cargo tree -p alife_core`: PASS; no Bevy/wgpu/app/Blender dependency leak.
- `cargo check --workspace --all-features --all-targets`: PASS.
- `cargo test --workspace --all-features --all-targets`: PASS.

## Boundaries Preserved

- `alife_core` unchanged.
- No Bevy/wgpu/app dependency leaked into `alife_core`.
- Blender remains offline art tooling, not a runtime dependency.
- The asset pipeline emits no actions, no cognition state, no semantic output, and no weight updates.
- CPU fallback unchanged.
- CPU shadow parity unchanged.
- No full action-authoritative GPU claim.
- No S12/G25/P37.
- No release tag.
- No screenshots, logs, target artifacts, model files, caches, receipts, or generated probe outputs tracked.

## Known Limitations

- This slice does not implement the post-processing render pass:
  - low-resolution downsample remains future work;
  - toon quantization remains future work;
  - Sobel depth/normal outline remains future work.
- This slice calibrates existing true 2.5D assets; it does not introduce new animation, new gameplay, new sensory authority, or new procedural-world simulation semantics.
- Blender is discovered locally but not on PATH. The wrapper handles this on this machine; other machines may need `BLENDER_EXE` or a standard Blender install.

## Next

- Continue the active visual pipeline goal with shader/post-processing or neurochemical visual-feedback slices.
- Roadmap continuation remains stopped until explicitly requested.
