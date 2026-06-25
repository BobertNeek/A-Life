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

## Iteration 2 - Direct graphical CLI Vulkan loader noise

Issue observed:
The PowerShell launcher avoided Vulkan overlay loader noise, but a direct
`cargo run ... graphical-playground` invocation could still inherit or choose a
Vulkan-capable wgpu instance before the launcher environment guard ran. On this
Windows machine, the GOG Galaxy overlay manifest then printed repeated
`wgpu_hal::vulkan::instance` `ERROR` lines even though the graphical GPU alpha
smoke completed successfully.

Fix made:
`alife_game_app` now applies the same Windows graphical launch policy inside
the `graphical-playground` CLI path before Bevy or the GPU runtime initializes.
Normal Windows graphical alpha launches set `WGPU_BACKEND=dx12` for a clean
product path and apply the Vulkan loader log filter. Vulkan remains available
for diagnostics by setting `ALIFE_GRAPHICS_BACKEND=vulkan`, and
`ALIFE_GRAPHICS_BACKEND=existing` preserves an explicitly managed environment.

Command evidence:
- `WGPU_BACKEND=vulkan; cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/gpu_alpha --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 5` passed and printed `Windows graphical backend: WGPU_BACKEND=dx12 for clean alpha launch` without repeated Vulkan loader overlay errors.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5 -GpuMode static-plastic-cpu-shadow-guarded` passed without repeated Vulkan loader overlay errors.
- `cargo test -p alife_game_app --test app_shell first_graphical_alpha_playtest_docs_and_launcher_are_current -- --nocapture` passed and now checks the direct CLI guard.

Remaining player-facing problem:
The graphical alpha launch no longer looks broken from Vulkan overlay noise.
The next player-facing issue remains making ordinary Space/N play avoid or
recover cleanly from terminal-invalid states during longer manual sessions.

Screenshots/video:
No media committed. Any screenshots remain under ignored `target/` evidence
directories.
