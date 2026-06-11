# Decision log

Record architecture decisions here. Do not record every tiny implementation detail. Record decisions that affect public contracts, dependency boundaries, schemas, runtime behavior, or future plans.

| Date | Plan | Decision | Rationale | Alternatives rejected | Follow-up |
|---|---|---|---|---|---|
| 2026-06-10 | seed | Keep `alife_core` engine-independent | Core cognition must be testable without Bevy/GPU | Direct Bevy Entity/Vec3/Quat in core | Enforce in P01/P02 |
| 2026-06-10 | seed | Split runtime `ExperiencePatch` from packed logging | Runtime cognition and binary export have conflicting ownership/layout needs | One struct for everything | Implement in P10/P11 |
| 2026-06-10 | P00 | Normalize the plan pack to `docs/codex_plan_pack/` | The master prompt hardcodes this path and future branches need one stable location | Leaving the pack only at `docs/alife_codex_plan_pack_v1/` | Audit and commit stable pack path in P00 |
| 2026-06-10 | P01 | Remove tracked `a_life_revised_spec_pack/` mirror | It duplicated the active workspace scaffold and docs, increasing merge and validation noise | Archiving duplicate code under docs/specs | Keep source-of-truth specs in `docs/` |
| 2026-06-10 | P01 | Use `scripts/graphify.sh hook-check` for Codex Graphify hook | Hooks must be portable and optional; the script already skips cleanly when Graphify is unavailable | Absolute Windows user-profile Graphify executable path | P01 validation records WSL bash limitation and Git Bash substitute |
| 2026-06-10 | P02 | Put schema/versioning conventions under `docs/architecture/` | Version rules apply across ABI, logs, GPU layouts, saves, and offline tools, so they need a stable architecture home | Burying version rules only in plan receipts | P04/P11/P24/P30/P34 must follow this convention |
| 2026-06-10 | P02 | Use `alife_tools` integration tests for repo-level invariant harness | The tooling crate can validate docs/progress/file hygiene without polluting `alife_core` domain contracts | Root integration tests without a root package | Future plans can add repo invariant tests here |
