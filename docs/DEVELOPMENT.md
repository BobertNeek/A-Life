# Development

## Environment

A-Life is a Rust workspace targeting Windows, Bevy 0.18, wgpu 29, Vulkan, and WGSL. Install a current Rust toolchain and Git for Windows. Repository PowerShell wrappers call Git Bash for shell checks and avoid accidental WSL use.

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
- The default graphics profile is `MinSpecComfort1080p`. `MinimumSettings30x30` is a graphics floor; cognition still uses the GPU.

### Observe one production life across meals and sleep

`headless-max` already runs the admitted production runtime without lesson or
meal stops, including ordinary neural sleep, until the duration or `.stop` file
boundary, then writes and verifies its normal checkpoint. To observe that path,
set `ALIFE_ACTION_TRACE_PATH` to a new JSONL file before launch. Headless tracing
records every progressed world tick, including sleep and empty-population ticks;
checkpoint publication polls create no rows. No extra neural readback is used.

The default observation horizon is 10,000 ticks. Set
`ALIFE_HEADLESS_TRACE_MAX_TICKS` to an integer in `1..=1000000` for a longer bounded
trace. Reaching that limit or losing trace output does not end the life or prevent
the final checkpoint. The existing `.headless.json` receipt reports the trace's
first/last world tick, written count, limit, and any error. Compare its coverage
with the run's final tick before interpreting an unobserved wake. Rows stream to
disk rather than accumulating training captures in memory.

For a future authorized hardware observation, prepare a **disposable complete
copy** of an exact production save, its assets and durable manifest/journal,
matching config and asset manifest. Put the environment manifest at the absolute
path `COPY_ROOT/assets/environments/environment_manifest.json` and point its
scenario at the copied fixture root. Production derives its launch root from the
manifest's third ancestor, **not** the scenario's fixture root. With this layout,
runtime saves and UI settings go under `COPY_ROOT/target/artifacts/fvr05`; assets
go under the scenario's copied asset root. Copy the existing exact runtime save
and its hidden durable manifest/journal siblings into that isolated `fvr05`
directory, preserving their names. A shallow manifest in the checkout does not
isolate those output paths. Then run, for example, in PowerShell from the checkout:

```powershell
$env:ALIFE_ACTION_TRACE_PATH = 'target/artifacts/care-observation.jsonl'
$env:ALIFE_HEADLESS_TRACE_MAX_TICKS = '100000'
cargo run --release --locked -p alife_game_app --features production-voxel-frontend --bin alife_game_app -- production-voxel --manifest ABSOLUTE_COPY_ROOT/assets/environments/environment_manifest.json --scenario COPY_SCENARIO_ID --run-mode headless-max --smoke-seconds 120
```

The output directory must exist and the trace file must not exist. The raw CLI
accepts a duration in seconds; achieved world ticks depend on hardware. For a
longer observation omit `--smoke-seconds` and create the announced `.stop` file
to stop and checkpoint. A direct headless launch is required for a fresh trace;
switching from a renderer session that already opened the same path refuses to
overwrite that file and reports the trace error.

The runtime filename is
`PROFILE_SEED_SAVEIDHASH_runtime_save.json`, where `SAVEIDHASH` is the first 16
hex characters of BLAKE3 of the save ID. Use the same graphics profile and retain
the existing filename when copying. Without a matching runtime checkpoint the
frontend materializes a launch from the source save; this is not evidence of
exact continuation of the previously admitted life. The announced `.stop` path
must be inside the disposable root before letting this experiment proceed.

Follow the same `organism_id` and `stable_id` through the rows. `neural_sleep`
contains the actual scheduler phase, consolidation state, and cycle identity;
`body_sleeping` alone only describes the body. Action outcomes include measured
before/after physiology and per-channel physical evidence only when the sealed
patch matches this tick, organism, and sequence. Tick snapshots include current
`fatigue` and endocrine `sleep_pressure` even without waking action outcomes;
both inform waking readiness and are observations, not new success criteria.
Check exact Eat target and consumption separately from nourishment. Observe later Rest/fatigue relief,
consolidation, and an actual subsequent waking decision in temporal order;
there is no new composite success flag or compulsory training gate. Removed
organisms disappear from the organism snapshot; never substitute another life.

This path can observe a copied exact life only if that complete exact production
save exists. A foundation asset or CPU actor checkpoint cannot reconstruct its
body/private state. Starting with `--new-game --founder n2048:...` creates a fresh
life instead. Foundation evaluation retains its bounded contiguous replay and
sleep-censoring semantics. Offline world/scheduler fixtures use synthetic neural
selection/consolidation and do not establish autonomous eating, sleep, or waking
on a learned policy.

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

### Dependency security and texture compatibility

The frontend enables Bevy's `zstd_c` backend, sharing the workspace's existing
`zstd 0.13.3` dependency. Bevy 0.18.1's `zstd_rust` backend requires
`ruzstd ^0.8.0`, which cannot resolve the fix (`>=0.9.1`) for
[RUSTSEC-2026-0344](https://rustsec.org/advisories/RUSTSEC-2026-0344.html).
Keep KTX2 and tonemapping LUT support enabled when changing this backend.
The native CI matrix builds the production frontend on Linux and Windows and
tests valid pixel decoding and invalid/truncated zstd rejection through
`Image::from_buffer`, the public Bevy KTX2 path:

```text
cargo test --locked -p alife_game_app --features production-voxel-frontend --test bevy_texture_decode
```

The targeted lockfile fixes are:

| Package | Fixed version | Advisory and dependency exposure |
| --- | --- | --- |
| anyhow | 1.0.103 | [0190](https://rustsec.org/advisories/RUSTSEC-2026-0190.html): optional Hanabi VFX; affected operation is mutable error downcasting after context. |
| event-listener | 5.4.2 | [0221](https://rustsec.org/advisories/RUSTSEC-2026-0221.html): Bevy async infrastructure; affected operation uses stack listeners with non-Send tags. |
| memmap2 | 0.9.11 | [0186](https://rustsec.org/advisories/RUSTSEC-2026-0186.html): frontend font/window dependencies; affected operations take unchecked mapping ranges. |
| quick-xml | 0.41.0 | [0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html), [0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html): released `wayland-scanner 0.31.11` accepts `^0.41`, replacing the old `^0.39` restriction. |

Cargo's Linux production graph reaches quick-xml only through the
`wayland-scanner` proc macro, via Bevy's default platform → winit → Wayland
dependencies. It reads protocol XML bundled in the registry packages at build
time (for example, wayland-client's `wayland.xml` and wayland-protocols' bundled
protocols); it does not parse player or network XML at runtime. The Windows
production graph does not reach it. The old scanner used plain `Reader` and
attribute iteration, so the duplicate-attribute CPU issue was relevant to its
parser, while the namespace-specific `NsReader` issue was not demonstrated.

These were affected locked versions, not established exploits in A-Life. No
direct `downcast_mut`, mapping `advise_range`, or `Event::with_tag` call was
found in repository code; transitive affected calls have not been demonstrated.
The pre-fix frontend compiled the ruzstd KTX2 decoder, but the main tree had no
`.ktx2`/`.zst` assets and no public remote ingestion endpoint was established.
The small texture fixtures verify backend compatibility and error handling;
they do not establish a general decompression resource budget or malware
clearance. Reassess the resolved feature/platform graph on future updates.

## Focused persistence and asset checks

```powershell
cargo run -p alife_tools --bin p34_persistence -- validate-save crates/alife_world/tests/fixtures/p34/tiny_save.json crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants
```

Hardware checks use an available resource window. Report GPU unavailability as such.

## GPU measurements

Use the real device diagnostic for the workload being investigated:

```text
cargo run -p alife_tools --bin benchmark_tiers -- --backend gpu-closed-loop --targets TARGETS.json --classes CLASSES --sensor-profiles PROFILES --populations POPULATIONS --base-seed 4404 --output OUTPUT.json
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
