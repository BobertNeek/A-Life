# Global Invariants

These apply to every CA plan.

## Core boundaries

- `alife_core` remains engine-independent.
- No Bevy, Avian, wgpu, renderer, windowing, asset-loader, semantic-provider runtime, tool-only dependency, or UI type may enter `alife_core`.
- Stable IDs cross boundaries. Bevy `Entity`, renderer handles, GPU buffers, adapter IDs, and engine-local tokens do not enter portable saves or core contracts.
- GPU produces bounded summaries/deltas. Core owns validation and application.

## GPU truthfulness

- Player-facing default may be GPU-first.
- CPU fallback remains available unless a plan explicitly tests `RequireGpu` behavior.
- CPU shadow parity remains a correctness gate unless a later plan explicitly graduates to sampled/action-authoritative mode.
- Do not claim full action-authoritative GPU runtime until a plan proves it.
- CPU fallback is not GPU performance evidence.
- Do not add active bulk neural readback.
- Compact action summaries and post-seal boundary diagnostics are allowed only when bounded and documented.

## Cognition and learning

- ExperiencePatch sealing gates learning.
- H_shadow/lifetime deltas apply only through validated core-owned contracts.
- W_genetic_fixed is immutable by default.
- Lifetime-consolidated and H_operational layers are not mutated unless a plan explicitly proves and validates that path.
- Memory expectancy is bias/context only, not action replay.
- Topology/semantic/school/teacher systems cannot emit actions or bypass arbitration.
- Teacher and semantic systems are perception/context-only unless a later research plan adds a bounded non-authoritative adapter with review.

## Gameplay

- Bevy visuals mirror world/core state; they are not authoritative.
- World actions go through structured actions, arbitration, world execution, outcome observation, sealed patch.
- Visual effects, audio, UI, school cues, and semantic overlays cannot mutate cognition directly.
- Player controls must be observable and testable.

## Artifacts

- Do not commit screenshots, videos, logs, target artifacts, benchmark artifacts, generated captures, or large binary assets unless a plan explicitly creates a tiny versioned asset and validates it.
- Asset manifests must be versioned.
- Large content belongs outside git or in explicit release artifacts.

## Process

- Use Windows PowerShell wrappers.
- Never run plain `bash scripts/check.sh` on this Windows machine.
- Every implementation branch must include review and post-merge validation.
- Review gates stop for user/ChatGPT consultation unless the manifest explicitly permits continuing.
