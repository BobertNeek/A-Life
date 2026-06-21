# S00 Screenshot Index

Evidence directory: `target/playtest_evidence/S00/screenshots/`

Computer Use status: functional after the local runtime repair. App/window
enumeration, accessibility inspection, keyboard input, and Alt+PrintScreen
active-window screenshot fallback worked. The native WGC screenshot path remains
machine-limited on this Windows 10 build, so visual evidence uses the
Alt+PrintScreen clipboard fallback where a real target window exists. The A-Life
graphical smoke still left no persistent product window; unavailable A-Life UI
surfaces remain explicit placeholder evidence.

| screenshot | command/app state | what it proves | pass/fail/manual/unavailable | linked function(s) | notes |
|---|---|---|---|---|---|
| `target/playtest_evidence/S00/screenshots/001_desktop_before_launch.png` | Windows desktop before graphical launch | GUI session and screen capture fallback were available. | manual | environment baseline | Actual desktop capture, untracked under `target/`. |
| `target/playtest_evidence/S00/screenshots/002_launch_command_or_terminal.png` | Launch command evidence | The launch command evidence is in `graphical_real_launch_after_computer_use_repair.log`; terminal UI automation was intentionally avoided. | manual/unavailable | graphical playground real launch | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/003_initial_window_or_failure.png` | After `scripts/run_graphical_playground.ps1` | The graphical script completed but did not leave a persistent app window. | unavailable | bevy-smoke, graphical playground real launch | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/004_main_scene_visible.png` | Main scene check | No persistent main scene was visible; only CLI visible-world smoke evidence exists. | unavailable | visible-world-smoke | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/005_camera_or_view_state.png` | Camera/view check | No camera, pan, zoom, orbit, or view state could be exercised. | unavailable | graphical playground real launch | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/006_creature_visible_or_missing.png` | Creature visibility check | CLI creature visual smoke passed, but no rendered creature was visible. | unavailable | creature-visual-smoke | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/007_world_objects_food_hazard_visible_or_missing.png` | Food/hazard/resource visibility check | CLI world and ecology smokes passed, but no rendered world objects were inspectable. | unavailable | visible-world-smoke, world-ecology-loop-smoke | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/008_inspector_or_debug_overlay.png` | Inspector/debug overlay check | CLI inspector and cognition debug smokes passed, but no graphical overlay could be opened. | unavailable | creature-inspector-smoke, cognition-debug-smoke | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/009_save_load_or_menu_surface.png` | Save/load/menu check | CLI save/load UX smoke passed, but no graphical menu surface was available. | unavailable | save-load-ux-smoke | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/010_school_semantic_or_optional_demo_surface.png` | School/semantic UI check | CLI school and semantic smokes passed, but no graphical optional demo surface was available. | unavailable | school-mode-smoke, semantic-provider-smoke | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/011_gpu_fallback_or_status_surface.png` | GPU/status UI check | CLI GPU product smoke passed; no graphical GPU/status surface was available. | unavailable | gpu-product-smoke, benchmark_tiers --gpu-runtime | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/012_exit_or_shutdown_state.png` | Exit/shutdown check | The graphical command exited cleanly; no app process/window remained to close. | pass/manual | graphical playground real launch | Placeholder image. |
| `target/playtest_evidence/S00/screenshots/013_computer_use_alt_printscreen_active_window.png` | Repaired Computer Use screenshot fallback check | Computer Use keyboard input plus Alt+PrintScreen captured the active File Explorer window as a bounded window screenshot. | pass/manual | computer-use evidence only | This proves the screenshot fallback works; it is not A-Life product UI evidence. |
