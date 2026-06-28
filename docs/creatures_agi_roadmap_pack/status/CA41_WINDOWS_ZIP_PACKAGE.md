# CA41 - Windows zip packaging and run script

## Summary

CA41 adds a Windows alpha package builder for the GPU-first graphical
playground. The package is a local artifact only: it is generated under
`target/artifacts/ca41_windows_alpha/`, remains untracked, and does not create a
release tag.

The package carries:

- `alife_game_app.exe` built in release mode with `bevy-app gpu-runtime`.
- `run_windows_alpha_package.ps1`, a package-local runner that launches the EXE
  directly without Cargo.
- `crates/alife_game_app/environment_manifest.json`.
- `crates/alife_game_app/app_bundle_manifest.json`.
- `crates/alife_game_app/placeholder_art_manifest.json`.
- GPU alpha and P34 fixture configs, asset manifests, saves, and tiny reference
  weight fixtures.
- WGSL shader sources from `crates/alife_gpu_backend/shaders/`.
- `package_metadata.json` with commit, branch, package schema, product claim,
  package paths, and explicit no-release/no-full-action-authoritative flags.

## Commands

Dry-run the package builder without writing artifacts:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -DryRun
```

Build and assemble the local package:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1
```

Run the package after it is assembled:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/ca41_windows_alpha/alife-gpu-alpha-windows/run_windows_alpha_package.ps1
```

Run a bounded package smoke after assembly:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/ca41_windows_alpha/alife-gpu-alpha-windows/run_windows_alpha_package.ps1 -SmokeSeconds 30
```

## Invariants

- The player-facing package default is GPU-first:
  `static-plastic-cpu-shadow-guarded`.
- Product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- This is not full action-authoritative GPU runtime.
- CPU shadow parity remains the correctness gate.
- CPU fallback remains available and is described as safety/degraded mode.
- No release tag is created.
- No generated ZIP, package directory, screenshots, logs, target artifacts,
  model weights, model caches, or llama.cpp binaries are tracked.
- `alife_core` remains dependency-clean and engine-independent.

## Focused Evidence

CA41 focused checks run on the branch:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_windows_alpha_package.ps1 -DryRun
cargo test -p alife_game_app --test app_shell ca41 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo test -p alife_game_app --test app_shell platform_package -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/ca41_windows_alpha/alife-gpu-alpha-windows/run_windows_alpha_package.ps1 -DryRun -SmokeSeconds 30
```

Observed results before full validation:

- Package builder dry-run passed and listed the EXE, manifests, fixtures, WGSL
  shader directory, runner, and metadata writes without creating artifacts.
- Package-local runner dry-run passed before package assembly and reported
  missing package files as dry-run notes rather than hard failures.
- Focused CA41 script/doc test passed.
- Existing `platform-package-smoke` passed and now reports seven package
  commands including CA41 dry-runs.
- Release build/package assembly passed and produced ignored local output at
  `target/artifacts/ca41_windows_alpha/`.
- ZIP creation was verified at
  `target/artifacts/ca41_windows_alpha/alife-gpu-alpha-windows.zip`.
- The package builder rejects sibling paths such as `target/artifacts2/...`
  instead of accepting prefix matches.
- Assembled package runner dry-run passed with
  `alife_game_app.exe graphical-playground --manifest ... --scenario gpu-alpha
  --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 30`.
- `git ls-files target target/artifacts graphify-out models .cache` returned no
  tracked generated artifacts.

## Known Limitations

- The packaged graphical run still depends on local Windows graphics support.
- `RequireGpu` behavior remains an explicit launch option rather than the
  default because CPU fallback is preserved.
- Local model weights and llama.cpp servers are not bundled.
- The package is not signed, not installer-wrapped, and not a release artifact.

## Receipt Fields

- Plan: CA41
- Branch: `codex/CA41-windows-zip-packaging-and-run-script`
- Runtime code changed: no simulation behavior changed; packaging/tool metadata
  and tests changed.
- Core APIs changed: no.
- Docs changed: yes.
- Public APIs changed: no.
- Tests added/changed: CA41 script/doc/package checks in
  `crates/alife_game_app/tests/app_shell.rs`.
- Focused evidence: package dry-run, release package assembly, package runner
  dry-run, CA41 focused test, platform package smoke.
- Validation results: full branch validation passed:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`,
  `cargo test --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`,
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`,
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`,
  `cargo tree -p alife_core`,
  `cargo check --workspace --all-features --all-targets`, and
  `cargo test --workspace --all-features --all-targets`.
- Invariant checks: no S12/G25/P37, no release tag, no tracked generated
  artifacts, no `alife_core` dependency leak, no full action-authoritative GPU
  claim.
- Main status: pending merge.
- Next plan: CA42.
