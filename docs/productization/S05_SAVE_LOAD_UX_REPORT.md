# S05 Save/Load UX Report

Status: implemented on `codex/S05-save-load-menu-ux`.

## Summary

S05 exposes the existing P34/G15 save/load session state as a player-facing
menu surface in the graphical playground. The implementation reuses the
validated portable save, config, asset manifest, slot manager, overwrite guard,
and error-display code that already powered `save-load-ux-smoke`; it does not
change the save schema or runtime persistence boundary.

The graphical shell now adds an S05 overlay with:

- New / Save / Load / Settings tabs.
- Manual save and autosave slot lines.
- Stable world IDs and restored object count.
- Overwrite confirmation and cancel guidance.
- Readable load/config/asset error banner text.
- CPU fallback/backend and no-active-readback settings.
- Stable-ID-only boundary text with no engine-local ID serialization.

## Blueprint

An S05 UI blueprint image was generated locally and used as a visual reference
during implementation. The blueprint artifact is not committed. The implemented
surface is intentionally simpler than the blueprint: it is a debug-text Bevy
overlay, not a polished clickable menu. It matches the blueprint at the
information architecture level: tabs, slots, overwrite warning, error banner,
settings/backend status, and stable-ID policy.

## What Changed

- `player_save_load_menu_text` renders a product-facing text view from the
  existing `SaveLoadUxSmokeSummary`.
- The Bevy graphical playground spawns a display-only `SaveLoadMenuOverlay`
  from the same validated P34 fixture save/load state.
- `scripts/run_graphical_playground.ps1` now announces the visible S05 overlay
  in dry-run and launch output.
- Tests assert the menu text exposes save/load flows, readable errors, cancel
  guidance, CPU fallback, no active readback, stable IDs, and no Bevy `Entity`
  leakage.

## Evidence

Focused commands expected for this plan:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34
```

```powershell
cargo run -p alife_tools --bin p35_playground -- save-load crates/alife_world/tests/fixtures/p34
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10
```

Manual screenshot evidence captured on this Windows machine:

`target/playtest_evidence/S05/screenshots/s05_save_load_menu_overlay.png`

Computer Use native window screenshot capture hit the known Windows 10
`SetIsBorderRequired` failure, so the evidence image was captured with the
Alt+PrintScreen active-window fallback.

## Invariants

- `alife_core` remains engine-independent.
- The default headless path remains CI-safe.
- Bevy remains behind `bevy-app`.
- The S05 overlay is presentation only; it does not mutate cognition or bypass
  action arbitration.
- Portable saves continue to use P34 stable IDs, schema versions, manifest
  validation, and adapter remap policy.
- Engine-local IDs are not written to save JSON.

## Known Limitations

- The graphical menu is not yet a polished clickable UI. It is a visible
  player/tester overlay backed by the validated save/load session model.
- Save/load actions are still exercised through the existing smoke/session
  flow rather than a mouse-driven slot picker.
- The current fixture has one creature and one food object; broader save-slot
  behavior remains bounded to deterministic fixture coverage.
- GPU and graphics evidence remain local/manual; CPU fallback remains the
  validated supported path.

## Next Step

Proceed to S06 only after S05 is reviewed, merged, and main validation passes.
