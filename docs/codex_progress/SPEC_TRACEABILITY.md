# Spec traceability matrix

Codex must expand this as plans are implemented. Each row links a spec requirement to code, tests, and current status.

| Requirement | Source spec area | Owning plan | Code location | Test location | Status |
|---|---|---|---|---|---|
| Stable plan-pack operating model exists | Codex operating rules / plan pack | P00 | `docs/codex_plan_pack/`, `docs/codex_progress/` | `cargo metadata --no-deps`, repo audit | complete |
| Core brain has no Bevy/wgpu dependency | ExperiencePatch hardening / CPU-GPU split | P01-P04 | `alife_core` Cargo manifest and modules | `scripts/check_core_boundaries.sh` | complete for P01 scaffold gate |
| Local and CI validation gates exist | Codex operating rules / validation strategy | P01 | `scripts/check.sh`, `scripts/check_core_boundaries.sh`, `.github/workflows/ci.yml` | Git Bash `scripts/check.sh`, CI workflow commands | complete |
| Traceability/progress logs identify completed plans | Codex operating rules / plan pack | P02 | `docs/codex_progress/PLAN_PROGRESS.md`, `docs/codex_progress/SPEC_TRACEABILITY.md` | `crates/alife_tools/tests/repo_invariants.rs` | complete |
| Schema and ABI changes are versioned | Public schema versioning | P02/P04/P11/P24/P30/P34 | `docs/architecture/schema_versioning.md`, `crates/alife_core/src/version.rs` | `crates/alife_core/tests/abi_validation_errors.rs` | complete for shared registry; migrations pending future schemas |
| Forbidden Unity/C#/HLSL artifacts stay absent | Architecture non-goals | P02 | repository file tree | `crates/alife_tools/tests/repo_invariants.rs` | complete for scaffold gate |
| Dependency boundary checks are automated | CPU-GPU split / core purity | P02 | `scripts/check_core_boundaries.sh` | `scripts/check_core_boundaries.sh --self-test` | complete |
| Engine-independent IDs, math primitives, units, and adapter boundary exist | Core IDs/math primitives/stable adapter boundary | P03 | `crates/alife_core/src/ids.rs`, `math.rs`, `units.rs`, `adapter.rs`; `docs/architecture/core_adapter_boundary.md` | `crates/alife_core/tests/id_math_units.rs` | complete |
| Shared validation trait, validated wrapper, typed errors, and diagnostics exist | ABI versioning / validation framework / error model | P04 | `crates/alife_core/src/validation.rs`, `error.rs`, `diagnostics.rs` | `crates/alife_core/tests/abi_validation_errors.rs` | complete |
| Performance contract freezes active readback, action staging, VRAM ledger, sharding, budgets, cadence, and benchmark counters before P05-P09 | Performance profiling / CPU-GPU boundary / sparse memory planning | P04.5 | `docs/architecture/P04_5_performance_contract.md` | `crates/alife_tools/tests/performance_contract.rs` | complete |
| Scalable brain classes, not fixed 2048 | Scalable brain classes | P05 | `crates/alife_core/src/brain_class.rs`, `crates/alife_core/src/lobe.rs` | brain class/lobe tests | scaffold present; P05 pending hardening |
| Lobe layouts and routing masks are class-bucketed | Lobe layout generation / sparse routing | P05 | future routing/lobe modules | P05 tests | pending |
| Genome controls development and weight split | Genome and developmental encoding | P06 | `crates/alife_core/src/genome.rs` and future weight split modules | P06 tests | pending |
| Genetic weights are immutable under lifetime learning | Weight decomposition | P06/P16 | future weight split and sleep modules | P06/P16 tests | pending |
| Drives and hormones are bounded and reject invalid values | Neurochemistry and drive system | P07 | `crates/alife_core/src/chemistry.rs` | P07 validation tests | pending |
| Sensory/context ABI is versioned and engine-neutral | Sensory ABI / semantic context | P08 | `crates/alife_core/src/sensory_abi.rs` | `crates/alife_core/tests/sensory_abi_contexts.rs` | complete |
| Context streams capture atmospheric, light, energy, vocal, social, and optional environment inputs | Sensory ABI / context streams | P08 | `crates/alife_core/src/sensory_abi.rs` | `crates/alife_core/tests/sensory_abi_contexts.rs` | complete |
| Optional Gaussian and semantic references remain metadata, not renderer or SLM/runtime objects | Sensory ABI / semantic and Gaussian context boundary | P08 | `crates/alife_core/src/sensory_abi.rs` | `crates/alife_core/tests/sensory_abi_contexts.rs` | complete |
| ExperiencePatch is three-phase | ExperiencePatch contract | P10 | `experience.rs` | `experience` tests | pending |
| Runtime ExperiencePatch remains separate from packed logs | ExperiencePatch / packed logging split | P10/P11 | future runtime and packed log modules | P10/P11 tests | pending |
| Memory recall returns expectancy, not replay | Memory contract | P12 | `memory.rs` | `memory` tests | pending |
| Topological concept map tracks edges/simplexes/gaps | Topological cognitive map | P13 | future topology modules | P13 tests | pending |
| Action output is structured | Runtime spec action arbitration | P09 | `action.rs` | `action` tests | pending |
| Action arbitration records proposals and rejection trace | Action ABI and motor arbitration | P09 | future arbitration modules | P09 tests | pending |
| CPU reference precedes GPU | Runtime implementation sequence | P15/P24-P29 | `brain_reference.rs`, GPU crate | parity tests | pending |
| CPU sparse projection schema exists before GPU contracts | Sparse tensor storage / CPU reference | P14/P15 | future neural state modules | P14/P15 tests | pending |
| Headless behavior harness gates world behavior | Testing strategy / world harness | P17-P20 | future harness crates/modules | scenario, golden trace, benchmark tests | pending |
| GPU buffer layouts and WGSL contracts are parity-gated | GPU compute pipeline / WGSL rules | P24-P29 | `crates/alife_gpu_backend` future modules | CPU/GPU parity tests | pending |
| Active GPU gameplay avoids synchronous readback | GPU memory profiles / no-readback runtime | P29 | future GPU runtime integration | P29 performance tests | pending |
| External teacher teaches through perception only | Schooling boundary | P23 | `crates/alife_school` future modules | P23 tests | pending |
| Internal semantic prior is private and modulatory | Internal SLM / semantic prior layer | P22/P23 | `crates/alife_semantic` future modules | P22/P23 tests | pending |
| Offline tools are optional and not runtime prerequisites | Offline tools / Graphify / D2NWG | P30-P33 | `crates/alife_tools`, future offline tools | P30-P33 tests | pending |
| Save/load and lineage exports are versioned and migratable | Persistence and lineage export | P34 | future save/load modules | migration/rejection tests | pending |
| Product release requires soak/performance gates | Production hardening | P36 | CI, benchmarks, release docs | P36 release gate | pending |
