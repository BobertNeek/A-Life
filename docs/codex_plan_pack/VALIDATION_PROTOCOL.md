# Validation protocol

Run the narrowest validation command first, then widen.

## Default Rust commands

Use these when available:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

For feature-gated work, also run:

```bash
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

## Boundary checks

At minimum, after any plan touching `alife_core`, inspect dependencies:

```bash
cargo tree -p alife_core
```

`alife_core` must not contain Bevy, Avian, wgpu, renderer, ECS engine, Python, or OS windowing dependencies.

Suggested grep checks:

```bash
rg "bevy|avian|wgpu|Entity|Vec3|Quat|RenderDevice|RenderQueue" crates/alife_core src/alife_core alife_core -g '*.rs'
```

The grep may find allowed words in comments or tests that assert absence. If it finds actual type usage or dependencies in core, fix it.

## Determinism checks

For reference brain, world harness, and GPU parity plans:

- Use seeded RNG only.
- Store golden fixtures under `tests/fixtures/` or `crates/*/tests/fixtures/`.
- Test repeated runs with the same seed.
- Test rejection of NaN, non-monotonic ticks, invalid IDs, missing phase data, and out-of-bounds drives/hormones.

## GPU parity checks

GPU plans must compare against the CPU reference on small deterministic matrices before scaling up. Use exact equality for integer schema transforms and bounded tolerance for floating/quantized activation outputs. Document tolerance and why it is safe.

## Completion receipt

Every plan must end with this receipt:

```text
Completion receipt
Plan:
Branch:
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s):
```
