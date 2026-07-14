# A-Life Playground and Examples

P35 provides a headless-first playground surface plus optional integration demos.
The examples exercise the existing contracts instead of adding product release
hardening. P36 owns release gates, soak tests, packaging, and production policy.

## Quickstart

### Headless CPU playground smoke

Run the default headless playground smoke:

```bash
cargo run -p alife_tools --bin p35_playground -- run-headless crates/alife_world/tests/fixtures/p34
```

This loads the P34 tiny runtime config and asset manifest, runs the deterministic
food-seeking scenario through the CPU reference brain, seals experience patches,
and reports packed-log/debug summaries.

### Full headless example suite

Run the full headless example suite:

```bash
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
```

This exercises save/load, school verifier, semantic optional-provider handling,
GPU authority failure handling, and docs-manifest validation without requiring a renderer or GPU
device. The fake semantic provider is covered by the feature-gated command
below.

## Save, Config, and Assets

P35 consumes P34 portable files:

- `crates/alife_world/tests/fixtures/p34/tiny_config.json`
- `crates/alife_world/tests/fixtures/p34/tiny_save.json`
- `crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json`

Portable saves use stable IDs and asset references. Engine-local Bevy entities,
Avian handles, wgpu handles, renderer handles, and OS window handles are not
serialized. Bulk tensors remain asset references with digests.

Validate the docs/sample manifest:

```bash
cargo run -p alife_tools --bin p35_playground -- validate-manifest examples/p35/playground_manifest.json
```

## Headless Scenarios

The default playground path uses `alife_world` headless scenarios and the P15 CPU
reference brain. It supports a creature, food, hazards, sleep hooks, sealed patch
collection, packed logging summaries, and drive/hormone/action debug text.

## School and Teacher Demo

The school demo dispatches teacher events as ordinary perception and verifies
sealed patches. Teacher metadata cannot bypass action arbitration.

```bash
cargo run -p alife_tools --bin p35_playground -- school-demo
```

## Semantic/Gaussian Demo

### Semantic fake-provider demo

Semantic/Gaussian context is optional perceptual context. Missing providers are
tolerated by default. The fake provider demo creates optional context when the
tool feature is enabled, without making semantic data authoritative world truth.

```bash
cargo run -p alife_tools --features semantic-demo --bin p35_playground -- semantic-demo
```

## GPU Fallback Demo

GPU runtime remains optional. The P35 demo requests a static GPU backend with
simulated unavailable hardware, verifies CPU fallback, and keeps diagnostic
readback boundary scoped.

```bash
cargo run -p alife_tools --bin p35_playground -- gpu-authority
```

### GPU hardware diagnostics

Manual hardware diagnostics can be run when GPU support is available:

```bash
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

If the hardware or validation flags are not set, the report may honestly record
CPU fallback rather than GPU performance. Do not treat CPU fallback reports as
GPU performance claims. P29 is the source for measured or unknown GPU
performance status.

## Bevy/Avian Adapter Demo

### Bevy adapter smoke example

The Bevy adapter smoke example is optional and may require a graphics-capable
environment:

```bash
cargo run -p alife_bevy_adapter --example minimal_adapter
```

The adapter converts engine entities into stable IDs and does not move Bevy or
Avian types into `alife_core`.

## Data Flow

```text
P34 config/assets
  -> headless world/scenario
  -> CPU/GPU backend selection with CPU fallback
  -> creature tick and action arbitration
  -> sealed ExperiencePatch
  -> packed log summary
  -> offline tools or playground debug text
```

School events enter as sensory/perception cues. Semantic/Gaussian data enters as
optional context. Save/load uses stable IDs and remap tables rather than engine
handles.

## Adding Examples

- Add a sensory channel in `alife_core` ABI first, then adapt it in `alife_world`
  or an adapter crate.
- Add an action through the structured P09 `ActionCommand`/arbitration path and
  world legality checks.
- Add a lesson through `alife_school` perception events and sealed-patch
  verifiers.
- Add optional generated assets through a P34 manifest entry and a digest.
- Regenerate fixtures only intentionally, then update docs and tests together.

## Troubleshooting

- GPU unavailable: use CPU fallback and keep GPU hardware checks manual.
- Schema mismatch: P34 rejects incompatible saves/configs/manifests explicitly.
- Nondeterminism: compare P18/P19 scenario seeds and golden trace summaries.
- Bad generated assets: validate manifests and digests before startup.
- Dependency leak on Windows: run
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
  and `cargo tree -p alife_core`. On non-Windows systems, run the same boundary
  shell script through the platform shell.

## P36 Handoff

Remaining release work includes packaging, release gates, soak tests, product
performance evidence, and any graphical UX hardening. P35 does not claim those
are complete.
