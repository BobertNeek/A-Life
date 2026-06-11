# Spec traceability matrix

Codex must expand this as plans are implemented. Each row links a spec requirement to code, tests, and current status.

| Requirement | Source spec area | Owning plan | Code location | Test location | Status |
|---|---|---|---|---|---|
| Stable plan-pack operating model exists | Codex operating rules / plan pack | P00 | `docs/codex_plan_pack/`, `docs/codex_progress/` | `cargo metadata --no-deps`, repo audit | complete |
| Core brain has no Bevy/wgpu dependency | ExperiencePatch hardening / CPU-GPU split | P01-P04 | `alife_core` Cargo manifest and modules | boundary/dependency tests | pending |
| ExperiencePatch is three-phase | ExperiencePatch contract | P10 | `experience.rs` | `experience` tests | pending |
| Memory recall returns expectancy, not replay | Memory contract | P12 | `memory.rs` | `memory` tests | pending |
| Action output is structured | Runtime spec action arbitration | P09 | `action.rs` | `action` tests | pending |
| CPU reference precedes GPU | Runtime implementation sequence | P15/P24-P29 | `brain_reference.rs`, GPU crate | parity tests | pending |
