# Core Adapter Boundary

Status: v0 scaffold convention.

`alife_core` owns engine-independent cognition contracts only. It must not import Bevy, Avian, wgpu, renderer, OS-windowing, Python, or vendor LLM types.

Bevy `Entity` values map to `WorldEntityId` outside core, in adapter/world layers. Bevy math values map to `Vec2f`, `Vec3f`, `Quatf`, `Pose`, `Velocity`, or `Aabb` outside core. Gaussian or semantic adapter handles map to stable core ID wrappers before they cross into cognition.

Adapter crates should use local wrapper types when Rust orphan rules prevent direct trait implementations on external engine types. The core crate provides conversion traits as a stable boundary but does not depend on adapter crates.
