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

## Required Validation Wrappers

On Windows, use the wrapper scripts for repository validation:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

The wrappers force Git Bash and avoid accidental WSL invocation.
