# Playable Sim Game Phase Progress

| Plan | Branch | Status | Commit/PR | Validation | Next |
|---|---|---:|---|---|---|
| G00 | codex/G00-product-audit | complete | branch commit | passed default, wrapper, and all-features validation | G01 |
| G01 | codex/G01-game-app-shell | complete | 8d092b8 | full default, wrapper, core boundary, and all-features validation passed | G02 |
| G02 | codex/G02-visible-world-binding | complete | 89406d2 | full default, wrapper, core boundary, visible-world smoke, and all-features validation passed | G03 |
| G03 | codex/G03-live-brain-loop-bridge | complete | branch commit | full default, wrapper, core boundary, live-brain smoke, and all-features validation passed | G04 |
| G04 | codex/G04-creature-render-expression | complete | branch commit | full default, wrapper, core boundary, creature-visual smoke, Bevy visual component smoke, and all-features validation passed | G05 |
| G05 | codex/G05-camera-selection-inspector | complete | branch commit | full default, wrapper, core boundary, creature-inspector smoke, Bevy inspector resource smoke, and all-features validation passed | G06 |
| G06 | codex/G06-food-hazard-sleep-loop | complete | branch commit | full default, wrapper, core boundary, playable survival-loop smoke, and all-features validation passed | G07 |
| G07 | codex/G07-world-ecology | complete | branch commit | full default, wrapper, core boundary, world-ecology loop smoke, save/load ecology round-trip, and all-features validation passed | G08 |
| G08 | codex/G08-population-social-loop | complete | branch commit | full default, wrapper, core boundary, population-social loop smoke, deterministic schedule/cap tests, and all-features validation passed | G09 |
| G09 | codex/G09-life-cycle-lineage | complete | branch commit | full default, wrapper, core boundary, lifecycle-lineage smoke, birth/death/lineage save tests, and all-features validation passed | G10 |
| G10 | codex/G10-school-playable-mode | complete | branch commit | full default, wrapper, core boundary, school-mode smoke, teacher perception/no-bypass tests, school save round-trip, and all-features validation passed | G11 |
| G11 | codex/G11-semantic-slm-provider | complete | branch commit | full default, wrapper, core boundary, semantic-provider smoke, disabled/fake provider boundary tests, and all-features validation passed | G12 |
| G12 | codex/G12-gpu-product-hardening | complete | branch commit | full default, wrapper, core boundary, gpu-product smoke, CPU fallback/no-readback tests, manual hardware docs, and all-features validation passed | G13 |
| G13 | codex/G13-world-editing-tools | complete | branch commit | full default, wrapper, core boundary, world-editor smoke, stable-ID save/load round-trip, invalid edit/cap tests, and all-features validation passed | R13 |
| R13 | codex/R13-retrospective-product-boundary-review | complete | branch commit | retrospective review complete; verdict FIX_REQUIRED; G01-G13 boundaries passed, but `alife_game_app/src/lib.rs` requires behavior-preserving module split before G14 | R13 remediation |
| R13 remediation | codex/R13-module-split-remediation | complete | branch commit | behavior-preserving `alife_game_app` module split completed; public exports, CLI smoke commands, feature gates, and `alife_core` boundary preserved | G14 |
| G14 | codex/G14-cognition-visualization | complete | branch commit | full default, wrapper, core boundary, cognition-debug smoke, sealed-patch timeline/read-only/bias-only/no-readback tests, and all-features validation passed | G15 |
| G15 | codex/G15-save-load-ux | complete | branch commit | full default, wrapper, core boundary, save-load UX smoke, slot overwrite/error-display/config-menu tests, and all-features validation passed | G16 |
| G16 | codex/G16-content-authoring | complete | branch commit | full default, wrapper, core boundary, content-authoring validator, tiny content pack, missing asset, perception-only lesson, creature preset, and all-features validation passed | G17 |
| G17 | codex/G17-audio-vfx-polish | complete | branch commit | full default, wrapper, core boundary, feedback-polish smoke, sealed outcome cue mapping, optional asset fallback, and all-features validation passed | G18 |
| R18 | codex/R18-population-performance-review | pending | not started | review gate required after G18 before G19 | G19 |
| R23 | codex/R23-feature-complete-rc-review | pending | not started | review gate required after G23 before G24 | G24 |
| R24 | codex/R24-final-playable-sim-review | pending | not started | final playable-sim roadmap lock review after G24 | None |
