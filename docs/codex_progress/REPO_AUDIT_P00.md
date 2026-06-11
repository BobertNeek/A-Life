# P00 Repository Audit

Date: 2026-06-10

Plan: P00 - Operating model, repo audit, and plan wiring

Branch: `codex/P00-operating-model`

Base commit: `daa4025e217f817b8b52f87dcce8de9e6f9c58cc`

## Summary

The repository is a Rust workspace scaffold with the expected seven A-Life crates. The newly supplied plan pack was found under `docs/alife_codex_plan_pack_v1/` and normalized to `docs/codex_plan_pack/` so future branches can follow the master prompt without path overrides.

P00 made no runtime architecture changes. The only file-tree change outside progress documentation is the plan-pack path normalization.

## Workspace Members

`cargo metadata --no-deps --format-version 1` reports these workspace members:

| Crate | Path | Current role |
|---|---|---|
| `alife_core` | `crates/alife_core` | Engine-independent scaffold contracts, IDs, ABI markers, brain class, lobe, genome, chemistry, experience, action, lineage, and traits. |
| `alife_world` | `crates/alife_world` | Bevy-independent world/action-legality boundary scaffold. |
| `alife_gpu_backend` | `crates/alife_gpu_backend` | wgpu backend manifest and placeholder backend; no neural kernels. |
| `alife_bevy_adapter` | `crates/alife_bevy_adapter` | Minimal Bevy plugin adapter boundary. |
| `alife_school` | `crates/alife_school` | External teacher/channel contract scaffold. |
| `alife_semantic` | `crates/alife_semantic` | Internal private semantic-prior scaffold. |
| `alife_tools` | `crates/alife_tools` | Developer tooling manifest scaffold. |

## Dependencies by Crate

| Crate | Dependencies |
|---|---|
| `alife_core` | `bitflags`, `bytemuck` with derive, `serde` with derive, `smallvec`, `thiserror` |
| `alife_world` | `alife_core` |
| `alife_gpu_backend` | `alife_core`, `pollster`, `wgpu` |
| `alife_bevy_adapter` | `alife_core`, `bevy` with default features disabled |
| `alife_school` | `alife_core` |
| `alife_semantic` | `alife_core` |
| `alife_tools` | none |

Root workspace dependencies include Bevy `0.18.0`, wgpu `29.0.3`, serde, bytemuck, bitflags, smallvec, thiserror, and pollster.

## Current Scaffold State

Real contracts currently present in `alife_core`:

- Newtype IDs: `BrainClassId`, `GenomeId`, `LineageId`, `OrganismId`, `WorldEntityId`.
- Scalable brain class scaffold: `BrainScaleTier`, `BrainClassSpec`.
- Lobe scaffold: `LobeKind`, `LobeLayout`, `LobeRegion`.
- Genome scaffold: `BrainGenome`.
- Endocrine scaffold: `EndocrineProfile`.
- Experience scaffold: `ExperiencePatchHeader`, `ExperiencePatchPhase`.
- Action scaffold: `ActionCommand`, `ActionKind`, `ActionAbiVersion`.
- Sensory scaffold: `SensoryAbiVersion`, `TeacherPerceptionChannel`.
- Traits: `SemanticPriorProvider`, `SemanticPriorRequest`, `SemanticPriorPacket`, `NeuralComputeBackend`.
- Lineage scaffold: `LineageExportManifest`.

Placeholders or thin contracts that later plans should harden:

- `ExperiencePatchHeader` is not yet the rich three-phase runtime contract described by P10.
- `ActionCommand` is structured and versioned but does not yet include the full proposal/arbitration trace planned for P09.
- `BrainClassSpec`, `LobeLayout`, `BrainGenome`, and `EndocrineProfile` are scaffold-level and should be expanded by P05-P07.
- GPU backend exposes only a manifest and placeholder trait implementation; no WGSL kernels exist, which is correct before P24.
- World, school, semantic, and tools crates are boundary stubs only.

Tests currently present:

- `crates/alife_core/tests/scaffold_invariants.rs` contains seven scaffold invariant tests covering scalable reference tiers, disabled lobes, genome/endocrine basics, versioned experience/action headers, semantic/teacher boundary, scaffold traits, and lineage export metadata.

## Docs and Spec Files

Authoritative docs already exist:

- `AGENTS.md`
- `docs/AGENTS.md`
- `docs/master_spec.md`
- `docs/architecture_decisions.md`
- `docs/future_research_compatibility.md`
- `docs/schooling_and_teacher_architecture.md`
- `docs/codex_handoff_prompt.md`

P00 added progress artifacts:

- `docs/codex_progress/PLAN_PROGRESS.md`
- `docs/codex_progress/DECISION_LOG.md`
- `docs/codex_progress/SPEC_TRACEABILITY.md`
- `docs/codex_progress/REPO_AUDIT_P00.md`

## Scripts and CI

Scripts present:

- `scripts/setup.sh`
- `scripts/build.sh`
- `scripts/test.sh`
- `scripts/run.sh`
- `scripts/graphify.sh`
- `scripts/docs_check.sh`

CI files:

- No `.github` directory is present.

Tooling files:

- `.cargo/config.toml` sets `target-dir = "target"` and configures `wasm32-unknown-unknown` `getrandom_backend="wasm_js"`.
- `.codex/hooks.json` exists and points to an absolute Windows Graphify executable path.
- Project-scoped Graphify skill files exist under `.codex/skills/graphify/`.

## Obvious Hazards for P01

- `.codex/hooks.json` is not portable because it contains `C:\Users\PC\AppData\Roaming\uv\tools\graphifyy\Scripts\graphify.exe`.
- No CI workflow exists yet.
- `a_life_revised_spec_pack/` is a tracked nested scaffold/spec mirror and should be reviewed in P01 for removal or explicit retention.
- `graphify-out/` and `target/` are ignored generated directories and should remain outside normal commits.
- The first broad `.github` scan timed out; a narrower check confirmed no `.github` directory exists.

## Validation Availability

Cargo/Rust tooling is available. `cargo metadata --no-deps --format-version 1` ran successfully and produced the workspace summary above.

`cargo check --workspace --all-targets` ran successfully after the audit files were written.

## Initial and Expected Final Git Status

Initial P00 status before moving the plan pack:

```text
?? docs/alife_codex_plan_pack_v1/
```

Expected final status before commit:

```text
?? docs/codex_plan_pack/
?? docs/codex_progress/
```

Actual validation result:

```text
cargo metadata --no-deps --format-version 1: passed
cargo check --workspace --all-targets: passed
```
