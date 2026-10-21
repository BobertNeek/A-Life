# alife_gpu_backend

Own wgpu resources, WGSL packaging, sparse neural execution, and device state.
Use the shared runtime for consumers; keep world legality and presentation outside
this crate. Gameplay remains GPU-authoritative; device loss stops neural actions.
Opt-in offline CPU training is a separate execution path.

Validate host/shader layouts, buffer ranges, and slot ownership before dispatch.
Resolve logical addresses to local packed offsets without changing durable
identity. Preserve lifetime/fast weights, eligibility, and private recurrent state
across capture/restore. Complete or discard each outcome-bound learning
transaction before the next tick or retirement.

Keep training-only shaders and optimizer state outside normal game binaries and
saves. Measure real device work before claiming GPU correctness or performance.
