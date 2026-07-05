# G21 Platform Packaging and Local Smoke

G21 defines local packaging discipline and FVR08 applies it to the production
voxel desktop package. It does not publish a release, sign artifacts, create
installers, or create tags. Local build output belongs under
`target/artifacts/fvr08_windows_production` and remains untracked.

## Windows Headless Smoke

Use the PowerShell script from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1
```

The headless command runs the P35 playground through P34 fixture assets and
does not require GPU, graphics, Bevy windows, semantic provider hardware, or a
school UI.

## Windows Production Voxel Launch

The production voxel launch is manual because it opens a Bevy desktop window
and local graphics support can vary:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

The default profile is `MinSpecComfort1080p`. The minimum fallback profile is
`MinimumSettings30x30`, which is the 30-creature/30-FPS floor. GPU fallback is
diagnosed by the production launch preflight and reported explicitly; it is not
a fake GPU success claim.

## Non-Windows Smoke

On non-Windows systems, the equivalent helpers are:

```sh
./scripts/run_headless_playground.sh --dry-run
./scripts/run_headless_playground.sh
```

The production Windows package uses PowerShell because it is a Windows 10
desktop deliverable.

## Asset Bundle Manifest

The G21 bundle manifest is:

```text
examples/g21/platform_asset_bundle_manifest.json
```

It references only tiny committed fixtures and manifests. Bulk tensors, large
logs, GPU captures, generated benchmark reports, installer output, and local
package artifacts must stay out of git.

Validate the bundle discipline with:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
```

## FVR08 Windows Production Voxel ZIP Package

The FVR08 packaging pass builds a local Windows production voxel package. This
is still local artifact discipline: it does not publish, sign, tag, or create a
release.

Dry-run the builder:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1 -DryRun
```

Build a release EXE, copy the production manifests, production-named fixture,
production voxel asset pack, license bundle, WGSL shaders, and package-local
runner, then create a ZIP under `target/artifacts/`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

Run the assembled package without Cargo:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/fvr08_windows_production/alife-production-voxel-windows/run_windows_production_voxel_package.ps1
```

The package defaults to `MinSpecComfort1080p`, includes
`MinimumSettings30x30` as the fallback floor, requests
`auto-with-cpu-fallback`, keeps CPU fallback visible as degraded mode, preserves
CPU shadow parity, and does not claim full action-authoritative GPU runtime.
Crash summaries are written under
`diagnostics/fvr08_acceptance/crash_summary.md`.

## CA42 Runtime Prerequisite Preflight

The repository and package launchers run production launch preflight before
opening the graphical app. The preflight checks the requested GPU mode, probes
the local wgpu adapter when the `gpu-runtime` feature is enabled, records the
selected backend, reports fallback reason, and writes diagnostic output.

The legacy CA42 regression command remains `runtime-prereq-smoke` so old
preflight tests can still validate the `Runtime preflight log` path and GPU
failure messaging. It is a compatibility diagnostic, not the FVR08 product
launch or acceptance path.

Run it directly from the repository root:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- runtime-prereq-smoke --graphics-backend auto --log target/artifacts/ca42_runtime_prereq/runtime_prereq.log
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- production-voxel --profile MinSpecComfort1080p --record-performance
```

Use `-RequireGpu` only when deliberately testing GPU hardware. With
`-RequireGpu`, a CPU fallback becomes a clear GPU-unavailable failure instead of
a silent launch. Without `-RequireGpu`, CPU fallback remains available but is
reported as degraded mode. Neither path claims full action-authoritative GPU
runtime, and CPU shadow parity remains the correctness gate.

## Required Validation Wrappers

On Windows, use the wrapper scripts for repository validation:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

The wrappers force Git Bash and avoid accidental WSL invocation.
