# Alpha Layout and Control Evidence Fix Report

Status: post-evidence readability/control remediation.

## Scope

This fix addresses local Codex Computer Use medium findings from the first
graphical alpha evidence pass. It does not add gameplay systems, create
S12/G25/P37, create a release tag, or change the GPU runtime claim.

## Inspector Layout

The right read-only inspector/GPU runtime panel was compacted for the observed
1134x738 captured window size. It keeps these alpha-critical fields visible:

- selected stable ID
- selected action
- sealed patch status/count
- GPU mode
- selected backend or CPU fallback
- product claim
- CPU shadow gate
- H_shadow application count
- no full action-authoritative claim

The panel remains read-only and stable-ID based. It must not expose Bevy Entity
IDs in player-facing text.

## Hazard Visibility

The P34 tiny fixture displays creature and food markers. Hazard visibility is
now explicit as guide-only text in the graphical alpha overlay unless a richer
fixture supplies an actual hazard object. The docs no longer imply that P34
always contains a separate hazard marker.

## Control Evidence

Computer Use key injection failed during local evidence capture, so a
deterministic control verification command was added:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/p34
```

It verifies Space/N/1/2/3/F/Esc-equivalent semantics through the app control
surface without needing foreground keyboard injection. This is local
deterministic evidence, not a replacement for independent human alpha testing.

## Product Claim

The GPU claim remains:

```text
CpuShadowGuardedStaticPlusLiveHShadow
```

CPU shadow remains the gate. The app does not claim full action-authoritative
GPU runtime.

## External Evidence Status

Independent human external tester evidence is still missing. The previous
evidence remains local Codex Computer Use evidence.

## Release Status

No release tag was created. The next step remains collecting independent human
alpha tester evidence.
