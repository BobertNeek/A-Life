# True 2.5D Endocrine Tensor Feedback Proof

Classification: CA44A-ext-05 tensor proof

Branch: `codex/true25d-endocrine-tensor-feedback-proof`

This is a CA44A extension slice. It does not advance the CA roadmap. CA44
remains blocked until independent external tester evidence exists. CA45 was
not started. The next roadmap item remains CA44 after evidence, not CA45.

## Objective

Move the existing True 2.5D neurochemical presentation closer to the requested
Phase 5 endpoint by routing selected-creature presentation through a flat,
bounded endocrine tensor lane instead of only loose visual cue variables.

## Implementation Summary

- `CreatureVisualSnapshot` now carries the existing core
  `EndocrineSnapshot` from the live mind homeostasis.
- `GraphicalTrue25dFlatEndocrineTensor` records a fixed presentation tensor
  using `EndocrineSnapshot::to_array()` channel order.
- `GraphicalTrue25dEndocrineAssetFeedbackResource` now reports:
  - flat endocrine tensor channel count;
  - bounded tensor status;
  - tensor source;
  - dopamine biolume value;
  - bounded pain and low-hunger drive companions;
  - explicit no-action/no-weight tensor authority flags.
- Selected-creature root posture, cortisol/desaturation, dopamine/low-hunger
  biolume, and H_shadow learning presentation now derive through that tensor
  receipt.

## Boundaries

- Display-only.
- No action authority.
- No weight authority.
- No semantic, teacher, topology, memory, UI, or GPU bypass.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU claim.
- No active bulk neural readback.
- `alife_core` is not changed.

## Focused Evidence

Focused checks for this branch:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_creature_asset_feedback -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
```

Result: PASS. The focused endocrine asset-feedback test passed, and the
broader `true_25d` app-shell group passed with 10 tests.

Graphical smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS. Local preflight selected `GpuPlastic` on the NVIDIA GeForce RTX
3050 Vulkan path with fallback `None`; the bounded graphical smoke exited
cleanly.

Forced fallback smoke:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. The smoke selected `CpuReference` with
`HardwareUnavailable`, kept the fallback explicit, and made no GPU performance
claim.

Full validation result: PASS on branch.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

Graphify update also passed with the local `graphify.exe`; updated graph
outputs remain ignored under `graphify-out/`.

## Known Limitations

- This adds an explicit flat endocrine presentation tensor lane, but it still
  does not edit Blender-authored animation clips or glTF material internals.
- Pain and low hunger are drive companions because the core endocrine tensor
  contains hormone channels; they remain bounded presentation-only inputs.
- This proof does not add independent external CA44 tester evidence.
- This proof does not start CA45 and does not request external tester evidence.

## Invariant Checks

- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.
- No Bevy/wgpu/app dependency leaked into `alife_core`.
- No action authority changed.
- No CPU fallback or CPU shadow parity weakening.

## Next

Continue only if explicitly instructed. The CA roadmap remains stopped at CA44
until independent external tester evidence is available.
