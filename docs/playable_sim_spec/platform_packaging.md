# G21 Platform Packaging and Local Smoke

G21 defines local packaging discipline only. It does not publish a release,
sign artifacts, create installers, or claim final product readiness. Local
build output belongs under `target/artifacts/g21_local_package` and remains
untracked.

## Windows Headless Smoke

Use the PowerShell script from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1
```

The headless command runs the P35 playground through P34 fixture assets and
does not require GPU, graphics, Bevy windows, semantic provider hardware, or a
school UI.

## Windows Graphical Smoke

The graphical smoke is manual because local graphics support can vary:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
```

Passing this smoke means the local feature-gated Bevy adapter path compiled and
ran far enough to build the visible-world smoke. It is not a GPU performance
claim and does not replace the headless CPU correctness path.

## Non-Windows Smoke

On non-Windows systems, the equivalent helpers are:

```sh
./scripts/run_headless_playground.sh --dry-run
./scripts/run_headless_playground.sh
./scripts/run_graphical_playground.sh --dry-run
./scripts/run_graphical_playground.sh
```

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

## CA41 Windows Alpha ZIP Package

The Creatures-to-AGI CA41 packaging pass adds a local Windows alpha package
builder. This is still a local artifact discipline: it does not publish,
sign, tag, or claim release readiness.

Dry-run the builder:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -DryRun
```

Build a release EXE, copy the app manifests, tiny fixtures, WGSL shaders, and
package-local runner, then create a ZIP under `target/artifacts/`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1
```

Run the assembled package without Cargo:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/ca41_windows_alpha/alife-gpu-alpha-windows/run_windows_alpha_package.ps1
```

The package defaults to GPU-first
`static-plastic-cpu-shadow-guarded`, keeps CPU fallback available as
safety/degraded mode, preserves CPU shadow parity, and does not claim full
action-authoritative GPU runtime.

## Required Validation Wrappers

On Windows, use the wrapper scripts for repository validation:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

The wrappers force Git Bash and avoid accidental WSL invocation.
