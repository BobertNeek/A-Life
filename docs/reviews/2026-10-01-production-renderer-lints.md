# Production renderer Clippy cleanup — 2026-10-01

The separate branch `codex/cloud-production-renderer-lints-20261001` starts at
the narrow CI repair `3912139597a3bb694b3c12b370a31cecc30c873a`. That repair removes
the unnecessary borrow of a noncapturing, Copy error closure in the curated
first-tick residency gate. Error creation remains lazy and later closure reuse
and all residency checks are preserved. The coordinator reported 1,575 passing
tests on its preceding candidate, with that Rust 1.99 lint as the sole CI failure.

The renderer branch resolves the 46 optional production library diagnostics and
seven further style diagnostics exposed when Clippy reached the test targets.
It uses standard Copy access, automatic dereferencing where the concrete type
allows it, `Option::as_deref`, `Path` parameters, lifetime elision, direct tail
expressions, default struct initialization, and unsigned `is_multiple_of` tests.
Quad extraction uses `as_chunks::<4>().0`, retaining the existing treatment of
complete quads and discarded incomplete tails. These APIs support the existing
1.96-era toolchain baseline.

Private ordinary helpers group existing closely related arguments or remove an
unused argument. Named camera/hand query filters and a panel-query tuple retain
the same ECS read/write access and disjointness. Thirteen parameter-count lint
exceptions apply only to individual Bevy systems/observers whose independent
arguments are injected by the framework; each includes that reason. There are
no module-wide suppressions or new ECS access patterns.

Test-only changes preserve every assertion: a boolean assertion is expressed
directly, single-element loops become equivalent scopes, fixed four-channel
colour comparisons use paired iterators, an unnecessary Copy clone is removed,
and one test module moves to the end of its file. The generic reset-runtime
argument retains its required explicit dereference. No fixture data, cognition,
training asset, persistence state, graphics asset, or shader is changed.

## Checks and limits

The saved cloud worker recovered with a clean candidate checkout and its previous
cache intact. Rust 1.99.0 was installed from the official distribution beside
the preserved Rust 1.98.1 toolchain; it is selected explicitly with `cargo +1.99.0`.

```bash
source /workspace/cloud-cpu-validation/env_graphics.sh
export CARGO_BUILD_JOBS=1
cargo +1.99.0 clippy --locked -p alife_game_app \
  --features 'production-voxel-frontend gpu-tests' --all-targets -- -D warnings
cargo +1.99.0 fmt --all -- --check
bash scripts/check.sh --quick
```

The full production-feature all-target strict Clippy check passes on Rust 1.99.0,
including library, library tests, binaries, and integration test compilation.
Formatting, static boundaries, whitespace, and documentation checks also pass.
This includes the explicit GPU test bodies without executing them. Compilation
used one job, incremental and dev/test debug information disabled, and the
existing 5 GiB free-disk/14 GiB cgroup-memory guards; none fired. Peak total
cgroup memory was approximately 12.28 GiB, with at least 13.21 GiB disk free.

No renderer/window, GPU test, desktop operation, training, or FPS measurement ran.
Compiled graphics test bodies are not reported as executed tests. The desired
20 FPS with 50 creatures is a separate performance goal; these source/style
repairs establish no population-scaling or hardware performance result.

Detailed local receipts remain in
`/workspace/A-Life/target/artifacts/cloud-repairs-graphics-candidate-e912bf0/`,
including `renderer_clippy_199_verified` and the narrow CI publication receipt.
Main and the original graphics, persistence, and frozen repair branches remain
unchanged. The renderer branch is published separately for review before its
validated source is combined with the reviewed memory/upload changes.
