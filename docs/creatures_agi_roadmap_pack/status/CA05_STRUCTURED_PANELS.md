# CA05 Structured Panels Status

Plan: CA05 - Mockup-inspired UI layer 1: structured panels

Branch: `codex/CA05-mockup-inspired-ui-structured-panels`

Status: implemented and validated.

## Product Change

The graphical alpha overlay now separates the player-facing surface into clearer
panels:

- left status panel for state, GPU mode, creature goal/action, patch count, and
  learning pulse status,
- right read-only inspector, still stable-ID based,
- bottom controls and visual legend panel,
- bottom event feed panel,
- footer boundary line for the CPU-shadow gate, product claim, no full
  action-authoritative claim, and no bulk readback status.

The old dense status overlay text remains available for existing tests and
diagnostics, but the Bevy graphical shell writes the structured status and event
feed into separate panels.

## Evidence

Focused tests were added for:

- structured status panel content,
- event feed separation,
- controls/legend panel wording,
- CPU-shadow boundary footer wording,
- no Bevy `Entity` IDs in the new player-facing text.

Local graphical smoke and forced CPU fallback smoke passed after the panel split.
An untracked Alt+PrintScreen visual capture under `target/ca05_visual_evidence/`
confirmed the status panel, inspector, controls panel, event feed, and
CPU-shadow footer are visible at the local Windows capture size.

## Boundaries

- `alife_core` was not changed.
- No Bevy, wgpu, GPU, or renderer dependencies were added to `alife_core`.
- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- The UI still states that CPU shadow remains the gate and that the runtime is
  not full action-authoritative.
- No screenshots, logs, target artifacts, release tags, S12, G25, or P37 were
  created by this plan.

## Known Limitations

CA05 is a first structured-panel pass. It improves layout and claim clarity, but
does not add new gameplay systems, new GPU runtime authority, or a release-ready
art direction pass.
