# CAR31 Review Report - Cognition Inspection

## Verdict

PASS_WITH_NOTES

No blocker, high, or medium findings remain for the CA28-CA31 tranche. CA32
must not start until the required user/ChatGPT consultation accepts this
hard-stop review.

## Scope Reviewed

- CA28 topological concept overlay.
- CA29 creature memory/history journal.
- CA30 neural activity and lobe profiler view.
- CA31 player lab tools for behavior comparison.
- Global invariants for inspection tools, stable IDs, action authority,
  read-only cognition surfaces, GPU truthfulness, model/runtime boundaries, and
  artifact hygiene.

## Files Inspected

- `docs/creatures_agi_roadmap_pack/review_gates/CAR31_cognition-inspection-review.md`
- `docs/creatures_agi_roadmap_pack/GLOBAL_INVARIANTS.md`
- `docs/creatures_agi_roadmap_pack/VALIDATION_PROTOCOL.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `docs/creatures_agi_roadmap_pack/status/CA28_TOPOLOGICAL_CONCEPT_OVERLAY.md`
- `docs/creatures_agi_roadmap_pack/status/CA29_MEMORY_HISTORY_JOURNAL.md`
- `docs/creatures_agi_roadmap_pack/status/CA30_NEURAL_ACTIVITY_PROFILER.md`
- `docs/creatures_agi_roadmap_pack/status/CA31_BEHAVIOR_COMPARISON_LAB.md`
- `crates/alife_game_app/src/topological_concept_overlay.rs`
- `crates/alife_game_app/src/memory_history_journal.rs`
- `crates/alife_game_app/src/neural_activity_profiler.rs`
- `crates/alife_game_app/src/behavior_comparison_lab.rs`
- `crates/alife_game_app/src/graphical_playground.rs`
- `crates/alife_game_app/src/interactive_runtime.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_core/Cargo.toml`

## Commands Run

Focused tranche evidence:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- topological-concept-overlay-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- memory-history-journal-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- neural-activity-profiler-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- behavior-comparison-lab-smoke --a gpu-alpha --b p34 --ticks 8 --out target/car31_behavior_comparison_report.md
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Boundary and artifact scans:

```powershell
git ls-files target models .cache
git tag --points-at HEAD
rg -n "S12|G25|P37|release tag|full action-authoritative|Entity\(|Ollama|api\.openai|api\.anthropic|Hugging Face Inference|cloud API|paid API" docs/creatures_agi_roadmap_pack/status docs/creatures_agi_roadmap_pack/reviews crates/alife_game_app/src crates/alife_game_app/tests/app_shell.rs
cargo tree -p alife_core
```

Standard validation protocol:

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

## Focused Results

- CA28 topological overlay smoke reported `nodes=5`, `edges=3`, `gaps=1`,
  `events=4`, `bypass_blocked=true`, and `direct_mutation=false`.
- CA29 memory journal smoke reported `memories=5`, `patches=5`,
  `bias_rows=4`, `action_replay_blocked=true`, and `direct_mutation=false`.
- CA30 neural profiler smoke reported `lobes=6`, `tiles=10/64`,
  `syn=640/8192`, `bulk_readback_blocked=true`,
  `action_authority_blocked=true`, and `weight_mutation_blocked=true`.
- CA31 behavior comparison smoke compared `gpu-alpha` with `p34` for 8 ticks,
  produced distinct bounded signatures, kept the export under the small-report
  cap, and reported no hidden training, semantic action, direct mutation, or GPU
  action-authority claim.
- Graphical GPU smoke selected `GpuPlastic`, used
  `CpuShadowGuardedStaticPlusLiveHShadow`, reported CPU shadow parity, showed
  CA28/CA29/CA30 signatures in the graphical output, and retained stable IDs.
- Forced fallback graphical smoke selected `CpuReference`, reported
  `HardwareUnavailable`, made no GPU claim, and continued sealed-patch evidence.

## Findings by Severity

### Blocker

None.

### High

None.

### Medium

None.

### Low / Notes

- CA28-CA31 are compact inspection panels and headless/report smokes, not full
  offline raw-trace analysis tools. Deeper raw neural, memory, topology, and
  behavior trace export remains appropriate future work.
- Graphical evidence is local smoke evidence. A human UX pass should still
  re-check readability of the combined Phase G panels before broader alpha
  distribution.

## Invariant Status

- `alife_core` remains engine-independent. `cargo tree -p alife_core` shows no
  Bevy, wgpu, renderer, model-runtime, semantic-provider, school-UI, or
  game-app dependency leak.
- CA28 topology remains bias/context only. It cannot emit actions, bypass P09
  arbitration, or mutate cognition.
- CA29 memory remains expectancy/context only. It cannot replay actions, emit
  actions, bypass arbitration, or mutate cognition.
- CA30 neural profiling uses compact summaries only. It does not add active
  bulk neural, per-lobe, per-synapse, or weight readback.
- CA31 behavior comparison runs isolated deterministic scenario copies and does
  not train, mutate the live runtime, inject semantic actions, or create GPU
  action-authority claims.
- Stable IDs remain the player-facing and portable boundary. Bevy `Entity`
  values are not exposed in reviewed player-facing text.
- CPU fallback and CPU shadow parity remain intact.
- Product GPU claim remains `CpuShadowGuardedStaticPlusLiveHShadow`; this
  tranche does not claim full action-authoritative GPU runtime.
- Active model/runtime status remains direct localhost-only llama.cpp. This
  tranche does not reintroduce active Ollama, cloud APIs, paid APIs, fake
  providers, or remote hosted inference.
- No screenshots, logs, target artifacts, model weights, model caches, `S12`,
  `G25`, `P37`, or release tag were created or tracked.

## User-Facing Status

The Phase G cognition inspection tools are useful as read-only player/developer
inspection surfaces:

- Topology exposes concept nodes, edges, gaps, and recent topology-linked events
  without action authority.
- Memory exposes recent sealed patches, memory records, and expectancy bias
  rows without action replay.
- Neural profiling exposes lobe, tile, synapse, route, backend, and compact
  readback status without bulk neural trace reads.
- Behavior comparison lets players compare two bounded scenarios through
  deterministic signatures and a small local report.

The graphical smoke demonstrates that these Phase G signatures can coexist with
the GPU alpha shell and forced CPU fallback path.

## Evidence Gaps

- No independent human UX pass was performed during CAR31.
- Phase G tools do not export full raw neural tensors, bulk memory histories, or
  full topology traces. Those remain future offline/export concerns.
- The graphical smoke is local machine evidence and should be treated as alpha
  smoke evidence, not a public release readiness claim.

## Fix Prompt if Needed

No fix prompt required. No blocker, high, or medium findings were found.

## Next Plan Recommendation

Stop at CAR31 for user/ChatGPT consultation. If this `PASS_WITH_NOTES` verdict
is accepted, the next manifest item is CA32 - Real-time WGSL telemetry in app.
Do not start CA32 until explicitly approved.
