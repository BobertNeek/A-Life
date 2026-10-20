# Current status

Baseline status: 2026-09-02. Care/training update: 2026-09-24.
The older capability matrix below is a historical baseline, not a fresh HEAD validation.

## Current care and training work

- **Signed Decoders & World-Model-to-Motor Tracts**: `ProjectionType::MotorProposal` (`TemporalPredictive -> ActionPlanning`) and `ActionCandidate` / `SpeechPayload` decoders support signed (excitatory and inhibitory) weights in both phenotype compilation and GPU foundation training, with negative default distance weights so closer candidates score higher by default.
- **Dedicated `PLAY` Motor Population**: `PLAY` (`ActionKind::Interact`) routes to `CandidateActionFamily::Other` (`DRIVE_BOREDOM_MASK | DRIVE_CURIOSITY_MASK`), decoupled from `GRAB` (`CandidateActionFamily::Contact`).
- **Credit-Gated Waking Oja Normalization**: Waking fast-weight homeostatic decay in `closed_loop_plasticity.wgsl` is gated by `clamp(abs(local_third_factor * credited_eligibility), 0.0, 1.0)` so unrewarded activity does not erase inherited or sleep-consolidated weights.
- **Valence-Aware Innate Priors & Newborn Nociception**: `Approach` innate non-contact cues respect chemical/recalled valence (`0.0` for known hazards, `1.0` for food, `0.75` neutral), `Avoid` includes a non-contact spatial retreat fallback, and newborn nociception is enabled unconditionally.
- **Batched Multi-Creature GPU Training Capture**: Foundation training captures mutable GPU state across all resident organisms in two batched submissions per tick (`capture_training_state_batch`) with fast `zstd` replay streaming.

The legacy `N2048EvolutionHardener`, `N2048ActiveBatteryRunner`, and `Era1TrialRunner` remain **Blocked** on canonical organism biology and receptor-gated learning integration. This does not block the foundation trainer or the production organism loop.

## Honest headline

The GPU-authoritative organism loop starts inside the production voxel application. The renderer seeds its records from the selected save, then consumes tick-bound authoritative frames for movement, births, selection, follow state, and retirement. The source path is causally connected. Current-source rendered lifecycle and performance proof remain outstanding.

The project is a research alpha. It is not ready for a public claim of an autonomous GPU voxel life game.

## Status vocabulary

- **Implemented** — the source contract or function exists.
- **Integrated** — the current `production-voxel` launch reaches it.
- **Player-visible** — the current Bevy scene or control surface exposes it.
- **Proven** — an executed causal receipt, hardware result, or rendered real-input check supports the claim.

## Capability matrix

| Capability | Implemented | Integrated | Player-visible | Proven now |
| --- | :---: | :---: | :---: | --- |
| World legality, unscored candidates, action execution, outcomes | Yes | Yes | Indirectly | Source-backed; prior bounded receipts |
| GPU closed-loop cognition and learning | Yes | Yes | Telemetry only | Source-bound EI receipts; not current-HEAD product proof |
| Save and GPU checkpoint contracts | Yes | Yes | Save surface | Source and prior round-trip receipts |
| Live world-to-voxel transforms | Yes | Yes | Yes | Source-backed; fresh rendered proof pending |
| Autonomous birth, ageing, reproduction, and death | Partial | Runtime presentation hooks active | Partial | Fresh autonomous lifecycle proof pending |
| Birth/death archive transactions | Yes | Runtime lifecycle path present | Retirement is visible | Source-backed; fresh transaction receipt pending |
| Teacher perception boundary | Yes | School smoke only | Diagnostic surfaces | Bounded smoke source, not live GPU school proof |
| Private local SLM prior | Yes | No live cognition call | No | Local inference smoke only |
| Optional SLM speech translation | Yes | Yes | Yes | Not exercised in this reset |
| Player speech into live world | Yes | Yes | Yes | Source-backed; not freshly exercised |
| Pause and load semantics | Yes | Yes | Yes | Source-backed; fresh real-input and fresh-process proof pending |
| Release package | Yes | Build path exists | Yes | Public release gates remain open |

## Production path today

1. `production-voxel` runs preflight and selects a source save.
2. The Bevy shell restores a required `GpuLiveBrainRuntime` and schedules its tick.
3. The GPU runtime gathers perception, enumerates unscored candidates, selects an action in WGSL, asks the world to execute it, applies the sealed result, and advances the world.
4. The renderer reconstructs initial creature records from the save.
5. Each changed presentation frame moves matching creature roots, creates missing newborn roots, and reconciles retired identities.
6. UI, selection, follow, contact shadows, and creature-attached effects read the same tick-bound presentation state.

The bridge is implemented and integrated in source. This status does not substitute for a fresh Vulkan run showing a complete birth-to-retirement sequence.

## Important control gaps

- Pause controls GPU scheduler playback in the integrated production path. Fresh real-input proof remains pending.
- Load validates the durable save, replaces the live runtime, resets scheduler state, and rebuilds the rendered scene. Fresh-process proof remains pending.
- Save can capture a real GPU checkpoint, but save/load/player-visible restoration still needs one fresh-process causal proof.
- The save/load UX autosave policy is 15 minutes of production simulation time. Sleep-journal publication is separate neural durability work, not a player save on every tick.
- Full autonomous lifecycle and lineage behavior still needs current-source causal and rendered proof.
- Dynamic overlay geometry is still derived from the launch snapshot.
- Sleep feedback reads each living individual's authoritative sleep phase and follows
  its displayed root pose. Three small, wordless amber bubbles clear on waking,
  missing authority, identity mismatch, removal and load; pause freezes their pulse.
  At most eight individuals share one mesh and material. This is presentation only;
  it does not change sleep, learning or persistence. Other scene VFX triggers remain
  launch-snapshot based, and stay disabled on authored terrain. Native gameplay
  visual acceptance remains unverified.
- Creature LOD is profile-selected rather than camera-distance selected.

## Evolutionary-intelligence results

### EI0 — bounded pass

EI0 passed its source-bound exit contract for lifecycle, archive, save/restore, tamper rejection, and a bounded GPU challenge battery. Era 1 was explicitly out of scope. The result does not prove general intelligence, autonomous product play, current-HEAD parity, or Era 1 readiness.

### EI1 — blocked

EI1 retained 2,640 source-bound descendant receipts and 52,800 GPU dispatches on an RTX 3050 through Vulkan. Its promotion verdict is `Blocked`. Recorded blockers are:

- `NonPositiveIntact`;
- `ControlNotWorseForEveryCell`;
- `ControlMarginBelowMinimum`.

Plateau status is `Unknown`; review eligibility and larger-brain authorization are false.

## Scale

Production insertion supports N512, N1024, and N2048. Fresh games default to the built-in Nano512 founder. N2048 care training is experimental and has not earned learned-care acceptance. N4096 is research-only. Higher enum values, population caps, and performance target tables are not evidence of production support.

## Release status

Not ready. Release requires autonomous lifecycle proof, truthful controls, current-source GPU and rendered evidence, target-hardware measurements, external playtest feedback, and an explicit release decision.
