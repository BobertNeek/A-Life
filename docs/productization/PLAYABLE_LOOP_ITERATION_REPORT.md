# Playable Loop Iteration Report

## Iteration 1 - Windows graphical launch Vulkan loader noise

Issue observed:
The player-facing graphical alpha launch could inherit `WGPU_BACKEND=vulkan` from an earlier diagnostic shell and then print repeated `wgpu_hal::vulkan::instance` messages from the GOG Galaxy injected overlay manifest. The app still ran, but the red `ERROR` lines made the launch look broken.

Fix made:
`scripts/run_graphical_playground.ps1` now has an explicit `-GraphicsBackend` option. On Windows, the default `auto` mode resolves to `dx12` and overrides inherited `WGPU_BACKEND` values for the normal alpha launch. Vulkan remains available through `-GraphicsBackend vulkan`, and `-GraphicsBackend existing` preserves the old environment-driven behavior for diagnostics.

Command evidence:
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5 -GpuMode static-plastic-cpu-shadow-guarded` passed with `WGPU_BACKEND=dx12`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded` passed with `WGPU_BACKEND=dx12`, `gpu_selected=GpuPlastic`, and `fallback=None`.
- `WGPU_BACKEND=vulkan; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 3 -GpuMode static-plastic-cpu-shadow-guarded` now prints `overriding inherited WGPU_BACKEND=vulkan with dx12` and passed without the repeated Vulkan loader overlay spam.
- `ALIFE_GPU_RUNTIME_AVAILABLE=0; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded` passed and reported `gpu_selected=CpuReference` with `fallback=HardwareUnavailable`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun -GraphicsBackend vulkan` confirms Vulkan remains an explicit diagnostic path.
- `cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/p34` passed.
- `cargo test -p alife_game_app --test app_shell first_graphical_alpha_playtest_docs_and_launcher_are_current -- --nocapture` passed.
- `cargo test -p alife_game_app s01_graphical_launcher_script_uses_persistent_window_commands -- --nocapture` passed.

Remaining player-facing problem:
The graphical alpha is GPU-first and readable, but first-user evidence should still focus on whether Space/N and event-feed changes feel active enough during manual play.

Screenshots/video:
No media committed. Any screenshots remain under ignored `target/` evidence directories.
