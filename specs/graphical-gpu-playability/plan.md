# Graphical GPU Playability Plan

1. Audit graphical launcher, Bevy shell, runtime controls, GPU live runtime, and reports.
2. Add a Bevy-free graphical GPU mode to launcher config.
3. Add an app-level graphical GPU controller that reuses the existing combined GPU static/plastic path.
4. Route Bevy runtime ticks and keyboard stepping through the controller.
5. Extend overlays, inspector text, marker color cues, CLI output, and the PowerShell launcher.
6. Add CI-safe tests for config, telemetry honesty, and stable-ID/read-only overlay output.
7. Run focused GPU/graphics commands, then full Windows-safe validation.
8. Perform strict review before merge.
