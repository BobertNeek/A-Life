# Repairs and island graphics integration candidate — 2026-10-01

`codex/cloud-repairs-graphics-candidate-20261001` is a separate review candidate
based on the frozen, coordinator-confirmed CI-green repair revision
`2bad5e02b6bbf125a10822d3c7effd7debc96d8f`. It combines the reviewed persistence
comparison and island graphics. It does not merge into main or include the
separately owned unfinished memory/timing work, trained founders, current actor
files, saves, or generated build output.

## Included ancestry

The graphics base `e76e9e12` and repair base share ancestor
`f8c9b35382f17c925cc6cab1083d18de6613e6b8`. Two main graphics commits are absent
from the repair base and are necessary before the final graphics refinement.
Each was applied once, without replacing the repair base with older main.

| Reviewed source | Candidate commit | Change |
| --- | --- | --- |
| `ba5e73cd5509bafbee7afca0f9f99d45c2641c2f` | `c5105af018bd70f41f7f706e591b7d1c1030819c` | Post-wake gameplay/learning comparison, explicit exact diagnostics |
| `c050013d5cefe4760b019e46cb0ebb37d774f300` | `b4a4bc64db6eb806cf8585beb568050f492b144f` | Approved island assets, world-owned player carry, wizard hand |
| `e76e9e123e7dac767a360e9703f365cb1b07d7f4` | `b8360da3c5604b7d5065a374bb7cc7fcd98bad4b` | Graphics review documentation |
| `fc0de5c376cefb1fed00ec4d94349dea777a38a1` | `e912bf0b0afd97d7bc3987098553c0d9be0f1da4` | Shore water and overview tree crowns |

All four cherry-picks applied without conflicts. Zero-context binary stable
patch IDs match their sources, confirming that their added/deleted content was
retained while existing repair context stayed in place. Source branches and the
frozen CI branch remain unchanged.

## Integration compatibility fixes

`cc95b68a1b4c23bbffe01c7d3e569feb598f1bfe` changes precisely five checked atomic
identifier allocations from `fetch_update` to `try_update`: four in
`closed_loop_buffers/bucket.rs`, one in `closed_loop_runtime.rs`. Installed
official Rust 1.98.1 source marks `try_update` stable since 1.95.0 and shows
`fetch_update` delegates to it; its deprecation starts in 1.99.0. This addresses
the coordinator's newer stable CI finding while preserving the documented
1.96-era compatibility, both relaxed orderings, checked increments, returned
previous values, and overflow errors. No warning suppression was added.

`fa342c8a7fc98a873fc756d9b3bd4e6429bb9de1` groups camera, hand, and selection
capture inputs in one derived Bevy `SystemParam`. The combined capture function
previously exceeded Bevy's supported function parameter count and failed system
registration compilation. Access, capture fields, scheduling, and behavior are
unchanged. It also removes the two private legacy selection-ring helpers that
the approved god-hand system no longer calls; the existing god-hand highlight
system still hides those legacy rings.

World legality, stable identity, grounded carry/release, and persistence remain
world-owned. Highlands saves retain their own terrain; new games select the
approved island. Shore textures and mipmaps are bounded startup allocations,
not extra per-frame geometry. These are scoped ownership/persistence checks
against AOA-INV-003, AOA-AUTH-001, AOA-WORLD-003, AOA-PERSIST-001/002/004, and
AOA-SLM-004, not a claim of complete architecture compliance.

## Validation

Tests run at the integrated picks and compatibility source; the final production
compile, strict CPU Clippy, and headless benchmark receipts bind code revision
`fa342c8a7fc98a873fc756d9b3bd4e6429bb9de1`. The subsequent report commit changes
documentation only. Earlier receipts record their source revision and working
tree status explicitly. There was no full workspace build/link/test repetition.

| Check | Result |
| --- | --- |
| `cargo test --locked -p alife_world --lib -- --test-threads=1` | **Pass: 67**, including island terrain and player carry/save regressions |
| `cargo test --locked -p alife_game_app --lib --features gpu-runtime -- --test-threads=1` | **Pass: 211**, including both persistence comparison regressions and production asset validation |
| `cargo test --locked -p alife_gpu_backend --lib -- --test-threads=1` | **Pass: 54** after the atomic replacements |
| `cargo test --locked -p alife_tools --test benchmark_tiers benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu -- --exact --test-threads=1` | **Pass: 1**, headless CPU smoke; 22 unrelated tests filtered |
| App, World, backend strict Clippy, all targets, `alife_game_app/gpu-runtime` | **Pass**, `-D warnings`; matches CPU workspace feature union |
| Same strict Clippy with `alife_game_app/gpu-tests` | **Pass**; explicit hardware test bodies compiled, not executed |
| App all-target check with `production-voxel-frontend gpu-tests` | **Pass** after the capture fix, including production assets and Hanabi |
| `python3 scripts/check_island_art.py` | **Pass**: terrain/LOD/collision/assets bounded and consistent |
| `scripts/check_island_canopies.py`, baseline from `e76e9e12` | **Pass**: near/middle geometry preserved; crown triangle budgets respected |
| `cargo fmt --all -- --check` and `bash scripts/check.sh --quick` | **Pass**, including static boundaries, whitespace, and documentation assertions |

The art check compared 201,960 authoritative terrain vertices, with maximum
height difference approximately `9.54e-7` metres, 1,716 collision proxies, 3,716
props, and five hand poses. The runtime art pack totals 33,277,445 bytes within
its 32 MiB budget. Canopy checks compare CPU geometry and silhouette coverage;
they establish no GPU rendering quality or frame-rate result.

Two failed attempts are preserved rather than counted as passes:

- Isolated app all-target Clippy without `gpu-runtime` cannot resolve the
  checkpoint fixture's feature-gated exports. That fixture and export boundary
  are unchanged from `2bad5e02`; `alife_tools` already enables `gpu-runtime` in
  workspace CI. The matching focused feature configuration passes.
- Optional production-feature strict Clippy remains **failed: 46 library
  diagnostics**, including renderer/UI system argument counts, complex query
  types, and style lints. These include inherited code and graphics code; this
  candidate does not claim that optional profile is lint-clean. The exact log is
  retained. No broad suppressions or unrelated renderer/UI refactoring were
  introduced to hide that result.

Physical GPU execution, Windows gameplay/visual review, FPS, full post-wake
hardware persistence, and training are **unrun**. No window or desktop was
launched. Hardware test registration is retained in the explicit profile.

## Cloud environment and reproducible setup

The usable worker is Debian 13.6, four CPU quota, 16 GiB cgroup memory without
swap, and a 32 GiB filesystem. Rust/Cargo are 1.98.1, rustfmt 1.9.0-stable,
Clippy 0.1.98. Lean CPU settings were sourced explicitly from
`/workspace/cloud-cpu-validation/env.sh`; this does not prove a saved UI setup
draft was automatically applied at boot. New shells still need those settings.

The renderer required `libudev-dev`, absent initially. Official Debian-signed
`libudev-dev` and `libudev1` 257.13-1~deb13u1 were extracted locally without root
or system file changes. The setup script was rerun successfully and a C
header/link/run check passed using `udev_new`/`udev_unref`, without enumerating
devices. Current renderer features do not require ALSA development files.

For this saved environment:

```bash
# Once when restoring a worker without the local native dependency:
bash /workspace/cloud-cpu-validation/setup-native-udev.sh

# In each build shell:
source /workspace/cloud-cpu-validation/env_graphics.sh
# env_graphics.sh also sources env.sh and exports local pkg-config/library paths.
# CPU builds use two jobs; production-feature compilation uses one:
export CARGO_BUILD_JOBS=1
```

The base environment exports `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`,
`CARGO_TARGET_DIR=/workspace/A-Life/target/cloud-cpu`, and
`RUST_TEST_THREADS=1`. `env_graphics.sh` also disables Python bytecode output.
Do not restore the earlier full-workspace link at startup.

Resource guards stop below 5 GiB disk free, above 14 GiB total cgroup memory,
or after 30 minutes per command. No guard fired. Observed peak across candidate
checks was 11.38 GiB total cgroup memory including cache; lowest observed free
disk was approximately 14.43 GiB. The work stayed on the cloud worker.

Detailed commands, logs, resource receipts, patch/source mappings, scope checks,
native setup receipts, and publication verification are local under
`/workspace/A-Life/target/artifacts/cloud-repairs-graphics-candidate-e912bf0/`.
Generated artifacts are excluded from the source publication. Publish only the
separate candidate branch to the existing repository; main integration and remote
CI assessment remain coordinator-owned.
