# CA43 - Crash Logs and Tester Feedback Capture

## Summary

CA43 adds tester-facing crash and feedback capture policy for the GPU alpha
launcher without committing local evidence artifacts. The repository and package
launchers now point testers at a local feedback directory, write a lightweight
feedback template before a run, and write a sanitized crash summary when
preflight or the graphical app exits with a nonzero code.

This is app/tooling and documentation behavior only. It does not change
simulation semantics, action authority, GPU/CPU correctness rules, or
`alife_core`.

## Files Changed

- `crates/alife_game_app/src/tester_feedback_capture.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/packaging_platform.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `scripts/run_graphical_playground.ps1`
- `scripts/run_windows_alpha_package.ps1`
- `docs/creatures_agi_roadmap_pack/templates/CA43_TESTER_FEEDBACK_TEMPLATE.md`
- `docs/creatures_agi_roadmap_pack/status/CA43_CRASH_LOGS_TESTER_FEEDBACK.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

## Runtime Code Changed

Yes, app/tooling only:

- added `tester-feedback-smoke`;
- added a versioned CA43 feedback-capture summary;
- added local path sanitization for crash summaries;
- added launcher-side feedback template and crash summary writing.

No gameplay, simulation, neural, save/load, action, semantic, SLM, or GPU
authority behavior changed.

## Public APIs Changed

App CLI:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- tester-feedback-smoke
```

Launcher output now prints:

- tester feedback directory;
- crash summary path on failure;
- feedback template path.

## Log Directory Policy

Repository launcher local evidence is written under:

```text
target/artifacts/ca43_tester_feedback/
```

Packaged runner local evidence is written under:

```text
diagnostics/ca43_tester_feedback/
```

These locations are local artifacts. They must not be committed.

## Crash Summary

On launcher failure, the scripts write:

```text
crash_summary.md
```

The crash summary records:

- schema;
- stage;
- exit code;
- sanitized command;
- related log path;
- explicit instruction not to commit media/log artifacts.

Local user and repository paths are replaced with placeholders.

## Feedback Template

The tracked source template is:

```text
docs/creatures_agi_roadmap_pack/templates/CA43_TESTER_FEEDBACK_TEMPLATE.md
```

The launchers copy a local template into the feedback directory for testers to
fill out beside their untracked evidence.

## Focused Evidence

Run:

```powershell
cargo test -p alife_game_app --test app_shell ca43 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- tester-feedback-smoke
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun -GpuMode static-plastic-cpu-shadow-guarded
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_windows_alpha_package.ps1 -DryRun -GpuMode static-plastic-cpu-shadow-guarded
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
```

If graphical behavior is smoke-tested:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Invariant Checks

- No S12, G25, P37, or hidden continuation plan.
- No release tag.
- No screenshots, videos, logs, captures, target artifacts, generated media,
  model weights, model caches, llama.cpp binaries, or downloaded model files
  are tracked.
- No Bevy, wgpu, GPU, model-runtime, UI, or app dependency added to
  `alife_core`.
- CPU fallback preserved.
- CPU shadow parity preserved.
- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- Full action-authoritative GPU runtime is not claimed.
- Semantic and SLM systems remain perception/context only.

## Known Limitations

- CA43 does not upload or aggregate tester evidence; it provides local capture
  policy and templates only.
- Screenshots and videos remain manually referenced, not committed.
- Crash summaries are lightweight triage artifacts and do not replace
  debugger-level crash dumps.

## Main Status

Branch validation passed. Pending merge to `main` and post-merge validation.

Next plan: CA44.
