# G18 Population Performance and LOD

G18 adds a product-facing population performance policy for the playable sim. It
does not change the CPU oracle, action arbitration, learning contracts, or GPU
runtime policy. The policy exists to keep visible population scale honest while
protecting sensory, motor, homeostasis, and action-arbitration cadence ahead of
nonessential cognition and presentation work.

## Target Tiers

| Population tier | Normal validation status | Evidence policy |
|---:|---|---|
| 1 | CI smoke | measured by default benchmark smoke |
| 10 | CI smoke | minimum playable population target |
| 50 | manual expected-slow | run manually before claiming product scale |
| 100 | manual expected-slow | run manually before claiming product scale |
| 250 | manual/GPU unknown | no GPU performance claim until measured |
| 500 | manual/GPU unknown | no GPU performance claim until measured |

The current G18 smoke records CPU/headless behavior and documents manual upper
tier commands. GPU performance remains unknown unless the manual GPU runtime
command is run on supported hardware and reports measured timings. CPU fallback
is valid behavior, but it is not GPU performance evidence.

## Commands

Headless product population smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- population-performance-smoke crates/alife_world/tests/fixtures/p34
```

Benchmark smoke tiers:

```powershell
cargo run -p alife_tools --bin benchmark_tiers
```

Manual upper tiers:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --all
```

Manual GPU runtime evidence, when supported hardware is available:

```powershell
$env:ALIFE_GPU_RUNTIME_BACKEND='static'
$env:ALIFE_GPU_RUNTIME_FEATURE='1'
$env:ALIFE_GPU_RUNTIME_AVAILABLE='1'
$env:ALIFE_GPU_RUNTIME_VALIDATED='1'
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

## LOD and Cadence

The G18 app policy uses three render residency bands:

| Residency | Population band | Visual detail | Animation cadence |
|---|---:|---|---:|
| hot | up to 10 | full | 60 Hz |
| warm | up to 100 | simplified | 20 Hz |
| cold | up to 500 | marker-only | 1 Hz |

Protected cadence targets keep sensory/motor, homeostasis, and action
arbitration at higher priority than nonessential cognition. When measured frame
pressure exceeds budget, nonessential cognition is decimated first. G18 tests
verify the LOD projection preserves the population behavior signature; it is a
presentation and cadence policy, not a gameplay shortcut.

## Handoff to R18

R18 must review whether this target policy is sufficient for G19 long-run
balance work and whether the manual tier/GPU evidence is still honest. If the
manual GPU command falls back to CPU, the result must be recorded as fallback,
not measured GPU performance.
