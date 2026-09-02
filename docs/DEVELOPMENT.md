# Development

## Environment

A-Life is a Rust workspace targeting Windows, Bevy 0.18, wgpu 29, Vulkan, and WGSL. Install a current Rust toolchain and Git for Windows. Repository PowerShell wrappers call Git Bash for shell gates and avoid accidental WSL use.

Read root `AGENTS.md` and the nearest subtree `AGENTS.md` before changing code or documentation.

## Launch and package

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

The default profile is `MinSpecComfort1080p`. `MinimumSettings30x30` is a graphics floor, not permission for CPU neural fallback.

## Standard checks

Run the smallest check that can falsify the changed behavior.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
```

Use a focused `cargo test -p <crate> <filter>` when a Rust behavior changes. Do not launch a second Cargo build while another shared-target build is active.

Release-oriented checks may also require:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

These commands are examples, not evidence by themselves. Record outputs under
ignored `target/artifacts/` paths when a gate requires a durable receipt.

## Focused content and persistence checks

Use the narrow validators for content and portable persistence:

```powershell
cargo run -p alife_tools --bin p34_persistence -- validate-save crates/alife_world/tests/fixtures/p34/tiny_save.json crates/alife_world/tests/fixtures/p34
cargo run -p alife_tools --bin g16_content_authoring -- validate-pack content/fixtures/g16/content_pack_manifest.json
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants
```

Validate the committed content pack with `validate-pack` before using it in a
tutorial or package. Optional GPU demonstrations remain manual.
The platform wrappers are:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

Optional systems must remain optional. A typed GPU unavailability result is a
failure state, not permission to substitute a reference brain.

## GPU gates

GPU claims require a physical adapter/backend receipt. Run the existing serialized hardware gate only when the source identity, worktree, process state, target path, and output path are explicit:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_gpu_closed_loop_gates.ps1
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Rules:

- require source-bound physical-adapter evidence for every hardware claim;
- do not count CPU, noop, fallback, preflight, or schema-only paths as GPU passes;
- bind commit, tree, adapter, backend, arguments, monotonic timing, exit code, and output digests;
- treat unavailable hardware or incomplete evidence as `Unknown` or `Blocked`;
- preserve source-bound caches and reports until their durable result is committed or explicitly discarded;
- never start a competing GPU corpus while one healthy run owns the target.

The formula-derived performance ledger used by its focused test lives at
`crates/alife_tools/tests/fixtures/P04_5_performance_contract.md`. The external
tester form packaged by the legacy alpha helper lives at
`examples/ca43/TESTER_FEEDBACK_TEMPLATE.md`. These are operational inputs, not
project documentation authorities.

## Architecture boundaries

- World code enumerates unscored candidates and owns legality and outcomes.
- Production neural results come from WGSL state.
- Presentation consumes read-only projections and cannot mutate world truth.
- Teacher cues enter through ordinary perception.
- The local SLM cannot act, target, reward, or write neural state.
- Archive ordering is part of the lifecycle transaction, not a best-effort side effect.

## Documentation changes

The active authority set is the eight documents linked from the root README plus operational `AGENTS.md` files. Update the relevant authority instead of adding a dated plan to the active docs tree. Git history retains superseded plans.

Before committing documentation:

```powershell
git diff --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

Check relative links and scan for stale claims such as production CPU fallback, EI1 promotion, N4096 production support, or a completed live GPU-to-voxel bridge.

## Repository hygiene

- Keep one worktree per active isolated task; remove it after its commits are merged and any uncommitted instruction edits are backed up.
- Keep durable reports in tracked report paths. Keep generated Cargo, graph, screenshot, log, and raw corpus output under ignored targets.
- Do not use `git clean`, force-push, destructive reset, branch deletion, reflog expiration, or Git garbage collection as routine cleanup.
- Stage intended files only. Preserve unrelated user work.
