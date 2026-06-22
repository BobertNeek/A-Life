# A-Life Productization Global Invariants

These invariants apply to every S-plan.

## Locked history

- P01-P36 are complete historical scaffold/release-gate work.
- G00-G24 and R24 are complete historical playable-sim roadmap work.
- Do not create P37, G25, or any automatic continuation chain.
- Future work must be explicitly authorized by the user or selected from backlog/productization docs.

## Core architecture

- `alife_core` remains engine-independent.
- Do not add Bevy, Avian, wgpu, renderer, ECS, OS-windowing, semantic-provider internals, school UI, or game-app state to `alife_core`.
- Headless CPU remains the correctness oracle and CI-safe fallback.
- Graphical/gameplay systems live in product/adapter crates.
- GPU remains optional with CPU fallback.
- Active gameplay must not require synchronous bulk neural readback.
- Save/load must use stable IDs, versioned schemas, P34 asset/config policy, and adapter remap where needed.
- Engine-local IDs must not be serialized into portable saves.

## Cognitive safety

- Teacher/school signals remain perception/context/feedback only.
- Semantic/SLM/Gaussian providers cannot issue actions, rewrite weights, or become world truth.
- Memory is expectancy/bias only, never action replay.
- Topology/curiosity cannot bypass action arbitration.
- `W_genetic_fixed` remains immutable by default.
- All learning-relevant values must reject NaN/out-of-range values.
- Sealed three-phase `ExperiencePatch` precedes learning/memory/topology/logging.

## Product evidence

- Do not claim a graphical game function works unless a graphical/manual/Computer Use run verified it, or it is clearly marked dry-run/manual/unavailable.
- Do not claim GPU performance from CPU fallback.
- Do not hide manual limitations.
- Do not commit large screenshots, videos, generated tensors, logs, GPU captures, benchmark artifacts, or `target/` outputs.
- Store evidence outputs under `target/playtest_evidence/...` and reference them from docs without tracking them.
- Prefer small, deterministic fixtures committed to the repo.

## Windows validation

- Do not run plain `bash scripts/check.sh` on Windows.
- Use PowerShell wrappers:
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
