# Development

## Environment

A-Life is a Rust workspace targeting Windows, Bevy 0.18, wgpu 29, Vulkan, and WGSL. Install a current Rust toolchain and Git for Windows. Repository PowerShell wrappers call Git Bash for shell gates and avoid accidental WSL use.

Run `bash scripts/setup.sh` to check the Rust, Git, Python, production-asset, Vulkan-tool, Blender, and optional Graphify environment. Read root `AGENTS.md` and the nearest subtree `AGENTS.md` before changing code or documentation.

## Launch and package

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -NewGame -Population 1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -PreviewCommand
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

- `-NewGame` starts a fresh creature with newborn nociception enabled. For the terrain N2048 founder candidate, add `-Founder 'n2048:assets/founders/terrain-care-n2048-v1/trained.alife-foundation'`.
- `-DryRun` executes application preflight without opening a window.
- `-PreviewCommand` prints the Cargo command without running it.
- The default graphics profile is `MinSpecComfort1080p`. `MinimumSettings30x30` is a graphics floor, not permission for CPU neural fallback.

### Controls & UI

- **Menu**: `Escape` or the menu button opens the pause menu (Play, Save, Load, Speed, Settings, Quit).
- **Camera**: Hold arrow keys or move the pointer near window edges to pan; `Home` snaps to the selected creature; `Page Up`/`Page Down` cycle creatures; `F` toggles follow; `R` resets view; mouse wheel zooms; `F12` saves a screenshot under `target/artifacts/player-captures`.
- **Care Dock & Speech**: Use the five-control care dock or `E` (place food), `C` (gentle touch), `J` (place toy), `K` (praise), and `Enter` (open speech composer). Select a creature and press `F2` (or click the pencil icon) to name it. `F1` toggles keyboard help; `F3` toggles debug rails and the textual inspector.

## Standard checks

Run the smallest check that can falsify the changed behavior:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1 --quick
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
```

Release-oriented workspace checks:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

Record the relevant commands and results with the change. Keep generated outputs under ignored `target/artifacts/` paths.

## Focused persistence and asset checks

```powershell
cargo run -p alife_tools --bin p34_persistence -- validate-save crates/alife_world/tests/fixtures/p34/tiny_save.json crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants
```

Optional GPU demonstrations remain manual. A typed GPU unavailability result is a failure state, not permission to substitute a reference brain.

## GPU measurements

Use the real device diagnostic for the workload being investigated:

```text
cargo run -p alife_tools --bin benchmark_tiers -- --backend gpu-closed-loop --targets TARGETS.json --classes CLASSES --sensor-profiles PROFILES --populations POPULATIONS --base-seed SEED --output OUTPUT.json
```

The command also supports `--write-canonical-targets PATH` and `--validate PATH`.
Choose classes and population rows relevant to the question; a target file is a
comparison input, not a requirement to rerun an entire promotion matrix. Record
the source, hardware, configuration, and observed result. CPU fixtures, estimates,
and preflight do not establish GPU timing or learned behavior. Keep unavailable
hardware and unmeasured behavior explicit.

## Architecture boundaries

- World code enumerates unscored candidates and owns legality and outcomes.
- Production neural results come from GPU-authoritative WGSL state.
- Presentation consumes read-only projections and cannot mutate world truth.
- Teacher cues enter through ordinary spatial perception.
- The local SLM cannot act, target, reward, or write neural state.
- Archive ordering is part of the lifecycle transaction, not a best-effort side effect.

## Optional code navigation

Graphify is optional and never a prerequisite for Cargo commands. On Windows, use `scripts/graphify.ps1`:

- `scripts/graphify.ps1 update --no-cluster` builds or refreshes the code graph.
- `scripts/graphify.ps1 query "<question>"`, `path "<A>" "<B>"`, or `explain "<concept>"` runs focused queries.
- Generated files under `graphify-out/` are ignored. Check source files directly when graph results are stale or incomplete.
