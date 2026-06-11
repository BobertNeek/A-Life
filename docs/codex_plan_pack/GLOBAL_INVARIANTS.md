# Global invariants Codex must preserve

These rules apply to every plan in this pack. They are not preferences. A plan is not complete if it violates any of them.

1. `alife_core` remains engine-independent. It must not depend on Bevy, Avian, wgpu, renderer resources, WGSL/HLSL buffer handles, OS windowing, ECS entity types, or Python runtime objects. Use stable IDs and pure Rust math/data contracts in core. Conversions belong in adapter crates.

2. Runtime cognition structs and packed logging structs stay separate. `ExperiencePatch` and related runtime snapshots may be rich Rust objects. `PackedExperienceFrame` must be fixed-size, versioned, intentionally lossy, and backed by side buffers for variable-length data.

3. Experience is causal and three-phase: pre-action state, decision, post-action outcome. No downstream component may learn from an incomplete or unordered event.

4. Memory recall returns expectancy bias, not action replay. Recall may affect salience, valence expectation, predicted drive deltas, affordance bias, danger/safety bias, and social trust/fear. It must not directly inject the old selected action into current motor output.

5. The genome/phenotype boundary stays explicit. `W_genetic_fixed` is immutable inherited baseline. Lifetime learning lives in `W_lifetime_consolidated`, `H_operational`, and `H_shadow`. Do not silently bake lifetime experience into genetic weights except behind an explicit experimental feature flag and test name.

6. GPU code accelerates math; it does not become the source of cognitive truth until CPU parity proves equivalence. Every GPU milestone needs a CPU reference comparison, a deterministic fixture, and documented acceptable error bounds.

7. Active gameplay GPU paths must not require synchronous device-to-host readback. Host-visible summaries are allowed only through planned staging/export points that do not stall the standard tick.

8. Flat sparse tensor work must use preallocated bounded buffers during active compute loops. Do not introduce dynamic allocation, pointer-chasing graph structures, or runtime resizing inside shader/active backend loops.

9. Teacher, school, semantic prior, and SLM features are modulatory/perceptual. They cannot bypass action arbitration or directly trigger motor commands.

10. Public schemas are versioned. Any breaking change to an ABI, packed format, saved-state format, or exported log requires a schema version bump, migration note, and tests for rejection or conversion.

11. All states that participate in learning reject NaN and out-of-range values. Drives and hormones must have explicit bounds and validation behavior.

12. Every plan must add or update tests. If a plan is pure documentation or infrastructure, it must add a validation script, CI gate, or traceability check instead.

13. Every plan must end with a completion receipt that lists files changed, commands run, tests passed/failed, deviations, and next plans.
