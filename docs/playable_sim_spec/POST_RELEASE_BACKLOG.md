# A-Life Playable Sim Post-Release Backlog

Status: backlog/issues notes only.

This file records future work after the G24 roadmap lock. It is not a new
implementation plan, G25, P37, or automatic Goal Mode chain. Any item here
requires a new explicit user instruction before implementation.

## Priority 0 - Release Evidence And Manual Gates

1. Record a real graphical playground smoke run on target hardware.
   - Current status: manual.
   - Command:
     ```powershell
     powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
     ```
   - Evidence required: OS, GPU/display context, command output, screenshots if
     visual verification is requested.

2. Record GPU hardware runtime/performance evidence.
   - Current status: adapter/device bring-up may be recorded locally by
     `benchmark_tiers --gpu-runtime`; bounded P25/P26 diagnostic timing may be
     recorded with `--measure-gpu`; a static CPU-shadow-guarded live-tick GPU
     smoke path may record product-smoke timing with compact readback. Full
     plastic live gameplay GPU timing remains manual/unknown until a safe
     post-seal lifetime-state update hook exists.
   - Command:
     ```powershell
     cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
     cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-action-authoritative --ticks 3
     ```
   - Evidence required: hardware identifier, backend status, fallback status,
     timing report, bottlenecks, and explicit 60 FPS target status. Environment
     flags can control fallback behavior but are not hardware proof by
     themselves.

3. Run extended headless soak and extended balance on release-candidate hardware.
   - Current status: ignored/manual to keep CI bounded.
   - Commands:
     ```powershell
     cargo test -p alife_world --test headless_soak -- --ignored --nocapture
     cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
     ```

## Priority 1 - Playability Polish

1. Tune non-scripted ecology balance with longer sessions.
   - Keep deterministic seeds and report degenerate behaviors honestly.
   - Do not overfit golden traces or hide failures.

2. Improve graphical UX after manual playtest evidence.
   - Use screenshots and explicit user feedback before changing visual behavior.
   - Keep headless CPU path default and feature-gated.

3. Expand tutorial/content packs with tiny versioned fixtures first.
   - Do not commit huge captures, logs, generated tensors, or asset dumps.
   - Keep school cues perception-only.

## Priority 2 - Optional Hardware And Packaging

1. Add packaging/signing automation only after explicit release policy approval.
   - G21 validates smoke and discipline, not final store packaging.

2. Promote selected manual GPU/graphics gates into CI only when hardware cost and
   availability are predictable.
   - Do not make GPU/graphics mandatory for default workspace validation.

3. Add richer diagnostics exports for offline analysis.
   - Keep active gameplay free of synchronous bulk neural readback.

## Priority 3 - Research/Tooling Follow-Ups

1. Improve ETF/neural-collapse activation exports when product diagnostics
   mature.
2. Expand generated weight/evolution lab experiments behind offline tooling.
3. Explore larger populations only with explicit performance evidence and
   bounded benchmark fixtures.

## Non-Goals For This Backlog

- No automatic G25.
- No P37.
- No hidden implementation phase.
- No release tag without explicit user approval.
- No `alife_core` dependency on Bevy, Avian, wgpu, renderer, ECS, semantic
  providers, school UI, or game-app state.
