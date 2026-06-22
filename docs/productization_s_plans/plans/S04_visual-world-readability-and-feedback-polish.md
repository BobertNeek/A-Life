# S04 - Visual world readability and feedback polish

Branch: `codex/S04-world-readability-feedback`

Dependencies:
- S03

Recommended model/reasoning: GPT-5.5 High

Next plan(s): S05

## Purpose

Improve visual readability of creatures, objects, terrain, outcomes, and feedback so a player can understand the sim without reading terminal logs.

## Owned scope

- presentation cues
- marker/placeholder shapes/colors
- outcome feedback cues
- basic VFX/audio mapping if already available
- readability report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_bevy_adapter/**
- docs/productization/**

## Forbidden scope

- full art pipeline
- huge assets
- new rendering engine
- gameplay balance changes unless display-only issue

## Implementation milestones

1. Make creature, food, hazard, obstacle, terrain/resource zones visually distinct.
2. Map sealed outcomes to non-authoritative feedback cues.
3. Display sleep/rest/pain/fear/curiosity/action intent states.
4. Add optional audio/VFX fallback cues if already supported.
5. Capture before/after screenshots.
6. Document remaining art gaps.

## Required tests and evidence

- feedback derives from sealed outcomes
- visual cues do not mutate cognition
- missing optional assets fallback safely
- no huge assets tracked

## Acceptance criteria

- A non-developer can visually distinguish creature, food, hazard, sleep/rest, pain/failure, and success states.
- Feedback does not control gameplay.
- No large assets are committed.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- feedback-polish-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- playable-survival-loop-smoke
```
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10
```

## Computer-use / manual evidence

- Screenshots of food, hazard, creature state, feedback cue, and sleep state.
- Create `S04_READABILITY_FEEDBACK_REPORT.md`.

## Failure handling

- If graphics unavailable, record manual evidence missing and produce CLI evidence only.
- If feedback becomes authoritative, revert to presentation-only mapping.

## Review checklist

- The plan implemented only `S04` scope.
- Runtime/code changes match the plan's owned scope.
- `alife_core` remains engine-independent.
- Headless CPU path remains green.
- Optional graphics/GPU/semantic/school systems remain optional unless explicitly hardened.
- No P37/G25/new automatic chain was created.
- Product claims match actual evidence.
- Reports under `docs/productization/` are honest about unavailable manual evidence.



## Global invariants

Read and obey:

- `docs/productization_s_plans/GLOBAL_INVARIANTS.md` if imported there, or the imported equivalent under the productization plan pack.
- Existing repo invariants in `AGENTS.md`.
- Existing P36/R24 validation discipline.

## Standard validation

Use Windows wrappers. Do not run plain `bash scripts/check.sh`.

Run the standard validation set from `VALIDATION_PROTOCOL.md`, plus each plan's focused commands.

## Completion receipt

```text
Completion receipt
Plan:
Branch:
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s):
Stopped:
```


## Required receipt override

```text
Completion receipt
Plan: S04 - Visual world readability and feedback polish
Branch: codex/S04-world-readability-feedback
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S05
Stopped: yes
```
