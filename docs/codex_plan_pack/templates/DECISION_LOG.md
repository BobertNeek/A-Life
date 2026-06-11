# Decision log

Record architecture decisions here. Do not record every tiny implementation detail. Record decisions that affect public contracts, dependency boundaries, schemas, runtime behavior, or future plans.

| Date | Plan | Decision | Rationale | Alternatives rejected | Follow-up |
|---|---|---|---|---|---|
| 2026-06-10 | seed | Keep `alife_core` engine-independent | Core cognition must be testable without Bevy/GPU | Direct Bevy Entity/Vec3/Quat in core | Enforce in P01/P02 |
| 2026-06-10 | seed | Split runtime `ExperiencePatch` from packed logging | Runtime cognition and binary export have conflicting ownership/layout needs | One struct for everything | Implement in P10/P11 |
